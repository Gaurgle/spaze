//! `spaze-client` — TUI client for the Spaze chat protocol.
//!
//! Phase 1.B: ratatui+crossterm TUI replacing 1.A's stdin/stdout. Two real
//! buffer kinds (Room, Help). Catppuccin Mocha theme.

use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures_util::{SinkExt, StreamExt};
use spaze_proto::{
    ClientCommand, ClientFrame, CommandOutcome, DeviceId, MessageBody, RequestId, ResponsePayload,
    RoomId, ServerEvent, ServerFrame, UserId,
};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{info, warn};

pub mod app;
pub mod buffers;
pub mod identity;
pub mod theme;
pub mod tui;

use crate::app::{App, ConnectionState, Identity, InputMode};
use crate::buffers::Buffer;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub server_url: String,
    pub user_id: UserId,
    pub device_id: DeviceId,
    pub display_name: String,
    pub room_id: RoomId,
}

/// Run the TUI client until `Ctrl+C` / stdin EOF / server close.
///
/// # Errors
///
/// Returns an error on connection failure or unrecoverable WS error.
#[allow(clippy::cognitive_complexity)] // tokio::select! main loop is inherently branchy
pub async fn run(config: ClientConfig) -> Result<()> {
    let (ws, _) = tokio_tungstenite::connect_async(&config.server_url)
        .await
        .with_context(|| format!("failed to connect to {}", config.server_url))?;
    info!("connected to {}", config.server_url);

    let (mut sink, mut stream) = ws.split();
    let identity = Identity {
        user_id: config.user_id,
        device_id: config.device_id,
        display_name: config.display_name.clone(),
    };
    let mut app = App::new(identity, config.server_url.clone());
    app.connection = ConnectionState::Connected {
        server_url: config.server_url.clone(),
    };

    let mut terminal = tui::setup()?;
    let mut events = EventStream::new();
    let request_counter = AtomicU64::new(1);

    // Initial draw.
    terminal.draw(|f| tui::draw(f, &mut app))?;

    let result: Result<()> = async {
        while !app.should_quit {
            tokio::select! {
                event = events.next() => {
                    match event {
                        Some(Ok(CrosstermEvent::Key(key))) => {
                            handle_key(key, &mut app, &mut sink, &config, &request_counter).await?;
                        }
                        // Resize triggers redraw at end of loop. Mouse/paste/focus
                        // events are ignored in 1.B.
                        Some(Ok(_)) => {}
                        Some(Err(err)) => {
                            warn!("crossterm event error: {err}");
                        }
                        None => {
                            // event stream ended (terminal closed)
                            app.should_quit = true;
                        }
                    }
                }
                incoming = stream.next() => {
                    match incoming {
                        Some(Ok(WsMessage::Text(text))) => {
                            handle_ws_text(&text, &mut app);
                        }
                        Some(Ok(WsMessage::Close(_))) => {
                            app.connection = ConnectionState::Disconnected {
                                reason: "server closed".to_string(),
                            };
                            app.should_quit = true;
                        }
                        Some(Ok(_)) => {}
                        Some(Err(err)) => {
                            app.connection = ConnectionState::Disconnected {
                                reason: format!("{err}"),
                            };
                            app.should_quit = true;
                        }
                        None => {
                            app.connection = ConnectionState::Disconnected {
                                reason: "stream ended".to_string(),
                            };
                            app.should_quit = true;
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    app.should_quit = true;
                }
            }
            terminal.draw(|f| tui::draw(f, &mut app))?;
        }
        Ok(())
    }
    .await;

    // Best-effort Close frame, run unconditionally so an inner-loop error
    // (e.g., handle_key WS write failure) doesn't skip it.
    let _ = sink.send(WsMessage::Close(None)).await;

    tui::teardown(terminal)?;

    // Print disconnect reason after teardown so stderr is back to normal.
    if let ConnectionState::Disconnected { reason } = &app.connection {
        eprintln!("connection lost: {reason}");
    }

    result
}

/// Handle a key event in the appropriate mode.
///
/// # Errors
///
/// Returns an error if serializing or sending a `ClientFrame` over the WS sink fails.
async fn handle_key<S>(
    key: KeyEvent,
    app: &mut App,
    sink: &mut S,
    config: &ClientConfig,
    request_counter: &AtomicU64,
) -> Result<()>
where
    S: SinkExt<WsMessage> + Unpin,
    <S as futures_util::Sink<WsMessage>>::Error: std::fmt::Display,
{
    // Insert mode: input box has focus.
    if matches!(app.mode, InputMode::Insert) {
        match key.code {
            KeyCode::Esc => {
                app.mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                if !app.input_buffer.is_empty() {
                    let frame = ClientFrame {
                        request_id: RequestId(request_counter.fetch_add(1, Ordering::Relaxed)),
                        command: ClientCommand::PostMessage {
                            room_id: config.room_id,
                            author_id: config.user_id,
                            author_device_id: config.device_id,
                            author_display_name: config.display_name.clone(),
                            body: MessageBody::Text {
                                content: app.input_buffer.clone(),
                            },
                        },
                    };
                    let json = serde_json::to_string(&frame).context("serialize ClientFrame")?;
                    sink.send(WsMessage::Text(json))
                        .await
                        .map_err(|e| anyhow::anyhow!("ws sink write: {e}"))?;
                    app.input_buffer.clear();
                }
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.input_buffer.push(c);
            }
            _ => {}
        }
        return Ok(());
    }

    // Normal mode: globals first, then per-buffer.
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match (key.code, ctrl) {
        (KeyCode::Char('c'), true) | (KeyCode::Char('q'), false) => {
            app.should_quit = true;
            return Ok(());
        }
        (KeyCode::Char('b'), true) => {
            app.toggle_sidebar();
            return Ok(());
        }
        (KeyCode::Char('i'), false) => {
            app.mode = InputMode::Insert;
            return Ok(());
        }
        (KeyCode::Char('?'), false) => {
            // Find help buffer, focus it.
            if let Some(idx) = app.find_buffer(|b| matches!(b, Buffer::Help(_))) {
                app.active = idx;
            }
            return Ok(());
        }
        (KeyCode::Tab, false) => {
            app.cycle_tab_forward();
            return Ok(());
        }
        (KeyCode::BackTab, _) => {
            app.cycle_tab_backward();
            return Ok(());
        }
        _ => {}
    }

    // Per-buffer.
    let _ = app.buffers[app.active].handle_key(key);
    Ok(())
}

fn handle_ws_text(text: &str, app: &mut App) {
    let frame: ServerFrame = match serde_json::from_str(text) {
        Ok(f) => f,
        Err(err) => {
            warn!("could not parse ServerFrame: {err}");
            return;
        }
    };
    match frame {
        // MessagePosted arrives via either the Response payload (when this
        // client is the sender) or as a pushed Event (when a peer posted).
        ServerFrame::Response {
            result:
                CommandOutcome::Ok {
                    payload: ResponsePayload::MessagePosted(m),
                },
            ..
        }
        | ServerFrame::Event(ServerEvent::MessagePosted(m)) => {
            push_to_room(app, m);
        }
        ServerFrame::Response {
            result: CommandOutcome::Err { error },
            ..
        } => {
            warn!("server response error: {error}");
        }
        // ResponsePayload::Empty + future variants, plus non-MessagePosted Events
        // — all no-op for 1.B.
        ServerFrame::Response {
            result: CommandOutcome::Ok { .. },
            ..
        }
        | ServerFrame::Event(_) => {}
    }
}

fn push_to_room(app: &mut App, msg: spaze_proto::Message) {
    if let Some(idx) = app.find_buffer(|b| matches!(b, Buffer::Room(_))) {
        if let Buffer::Room(rb) = &mut app.buffers[idx] {
            rb.push_message(msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::identity::derive_identity;

    #[test]
    fn same_name_yields_same_identity() {
        let (u1, d1) = derive_identity("andreas");
        let (u2, d2) = derive_identity("andreas");
        assert_eq!(u1, u2);
        assert_eq!(d1, d2);
    }

    #[test]
    fn different_names_yield_different_identities() {
        let (u1, _) = derive_identity("andreas");
        let (u2, _) = derive_identity("beth");
        assert_ne!(u1, u2);
    }

    #[test]
    fn empty_name_is_valid_and_deterministic() {
        let (a_user, a_device) = derive_identity("");
        let (b_user, b_device) = derive_identity("");
        assert_eq!(a_user, b_user);
        assert_eq!(a_device, b_device);
    }

    #[test]
    fn user_id_and_device_id_are_equal_in_phase_1a() {
        let (user, device) = derive_identity("andreas");
        assert_eq!(user.as_uuid(), device.as_uuid());
    }

    #[test]
    fn theme_catppuccin_mocha_has_all_slots_set() {
        use super::theme::Theme;
        use super::theme::catppuccin_mocha::CATPPUCCIN_MOCHA;
        use ratatui::style::Color;

        let default = Color::Reset;
        let t: Theme = CATPPUCCIN_MOCHA;
        assert_ne!(t.background, default);
        assert_ne!(t.surface, default);
        assert_ne!(t.overlay, default);
        assert_ne!(t.foreground, default);
        assert_ne!(t.muted, default);
        assert_ne!(t.subtle, default);
        assert_ne!(t.primary, default);
        assert_ne!(t.success, default);
        assert_ne!(t.warning, default);
        assert_ne!(t.error, default);
        assert_ne!(t.info, default);
        assert_ne!(t.mention, default);
        assert_ne!(t.link, default);
        assert_ne!(t.border, default);
        assert_ne!(t.border_focused, default);
        assert_ne!(t.tab_room, default);
        assert_ne!(t.tab_dm, default);
        assert_ne!(t.tab_special, default);
        assert_ne!(t.tab_unread, default);
        assert!(!t.syntect_theme_name.is_empty());
    }
}
