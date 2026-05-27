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
#[allow(clippy::too_many_lines)] // Insert-mode dispatch covers all InputKind variants + WS writes
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
            KeyCode::Enter if !app.input_buffer.is_empty() => {
                use spaze_commands::{
                    Effect, HandlerContext, InputKind, REGISTRY, lookup, parse_input,
                };

                let kind = parse_input(&app.input_buffer);
                match kind {
                    InputKind::Text(content) | InputKind::EscapedText(content) => {
                        let frame = ClientFrame {
                            request_id: RequestId(request_counter.fetch_add(1, Ordering::Relaxed)),
                            command: ClientCommand::PostMessage {
                                room_id: config.room_id,
                                author_id: config.user_id,
                                author_device_id: config.device_id,
                                author_display_name: config.display_name.clone(),
                                body: MessageBody::Text { content },
                            },
                        };
                        let json =
                            serde_json::to_string(&frame).context("serialize ClientFrame")?;
                        sink.send(WsMessage::Text(json))
                            .await
                            .map_err(|e| anyhow::anyhow!("ws sink write: {e}"))?;
                        app.input_buffer.clear();
                    }
                    InputKind::Command { name, args } => {
                        let Some(cmd) = lookup(&name, REGISTRY) else {
                            // Registry miss — leave input in bar, no submission,
                            // no system line. The visual red color was the feedback.
                            return Ok(());
                        };
                        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                        let ctx = HandlerContext { registry: REGISTRY };
                        let effects = (cmd.handler)(&arg_refs, &ctx);
                        for effect in effects {
                            match effect {
                                Effect::SendActionMessage(content) => {
                                    if !matches!(app.connection, ConnectionState::Connected { .. })
                                    {
                                        app.apply_effect(Effect::SystemLine(
                                            "not connected — /me requires an active connection"
                                                .into(),
                                        ));
                                        continue;
                                    }
                                    let frame = ClientFrame {
                                        request_id: RequestId(
                                            request_counter.fetch_add(1, Ordering::Relaxed),
                                        ),
                                        command: ClientCommand::PostMessage {
                                            room_id: config.room_id,
                                            author_id: config.user_id,
                                            author_device_id: config.device_id,
                                            author_display_name: config.display_name.clone(),
                                            body: MessageBody::Action { content },
                                        },
                                    };
                                    let json = serde_json::to_string(&frame)
                                        .context("serialize ClientFrame")?;
                                    sink.send(WsMessage::Text(json))
                                        .await
                                        .map_err(|e| anyhow::anyhow!("ws sink write: {e}"))?;
                                }
                                other => app.apply_effect(other),
                            }
                        }
                        app.input_buffer.clear();
                    }
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

    #[test]
    fn app_initializes_with_two_buffers() {
        use super::app::{App, Identity};
        use super::buffers::Buffer;
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".to_string(),
        };
        let app = App::new(identity, "ws://localhost".to_string());
        assert_eq!(app.buffers.len(), 2);
        assert_eq!(app.active, 0);
        assert!(matches!(app.buffers[0], Buffer::Room(_)));
        assert!(matches!(app.buffers[1], Buffer::Help(_)));
    }

    #[test]
    fn room_buffer_push_message_appends() {
        use super::buffers::room_timeline::{RoomKind, RoomTimelineBuffer};
        use spaze_proto::{DeviceId, Message, MessageBody, MessageId, RoomId, UserId};

        let mut rb = RoomTimelineBuffer::new(
            RoomId::new(),
            "# test".to_string(),
            RoomKind::Standard,
            "self".to_string(),
        );
        assert_eq!(rb.messages.len(), 0);
        let msg = Message {
            id: MessageId::new(),
            room_id: rb.room_id,
            author_id: UserId::new(),
            author_device_id: DeviceId::new(),
            author_display_name: "alice".to_string(),
            created_at_ms: 0,
            edited_at_ms: None,
            deleted_at_ms: None,
            body: MessageBody::Text {
                content: "hi".into(),
            },
        };
        rb.push_message(msg);
        assert_eq!(rb.messages.len(), 1);
    }

    #[test]
    fn room_buffer_scroll_sticks_to_bottom_on_new_message() {
        use super::buffers::room_timeline::{RoomKind, RoomTimelineBuffer};
        use spaze_proto::{DeviceId, Message, MessageBody, MessageId, RoomId, UserId};

        let mut rb = RoomTimelineBuffer::new(
            RoomId::new(),
            "# test".to_string(),
            RoomKind::Standard,
            "self".to_string(),
        );
        let room_id = rb.room_id;
        let make_msg = |content: &str| Message {
            id: MessageId::new(),
            room_id,
            author_id: UserId::new(),
            author_device_id: DeviceId::new(),
            author_display_name: "alice".to_string(),
            created_at_ms: 0,
            edited_at_ms: None,
            deleted_at_ms: None,
            body: MessageBody::Text {
                content: content.into(),
            },
        };

        rb.push_message(make_msg("a"));
        assert_eq!(rb.scroll.offset_from_bottom, 0);

        // User scrolls up.
        rb.scroll.page_up();
        assert!(rb.scroll.offset_from_bottom > 0);
        assert!(!rb.scroll.stuck_to_bottom);

        // New message arrives — viewport stays put (user has scrolled).
        let prior = rb.scroll.offset_from_bottom;
        rb.push_message(make_msg("b"));
        assert_eq!(rb.scroll.offset_from_bottom, prior);

        // User returns to bottom.
        rb.scroll.to_bottom();
        assert_eq!(rb.scroll.offset_from_bottom, 0);
        assert!(rb.scroll.stuck_to_bottom);

        // New message sticks to bottom.
        rb.push_message(make_msg("c"));
        assert_eq!(rb.scroll.offset_from_bottom, 0);
    }

    #[test]
    fn tab_cycle_wraps_around() {
        use super::app::{App, Identity};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".to_string(),
        };
        let mut app = App::new(identity, "ws://localhost".to_string());
        assert_eq!(app.active, 0);
        app.cycle_tab_forward();
        assert_eq!(app.active, 1);
        app.cycle_tab_forward();
        assert_eq!(app.active, 0);
        app.cycle_tab_backward();
        assert_eq!(app.active, 1);
        app.cycle_tab_backward();
        assert_eq!(app.active, 0);
    }

    #[test]
    fn help_buffer_scrolls_with_jk_and_arrows() {
        use super::buffers::HelpBuffer;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut hb = HelpBuffer::new();
        assert_eq!(hb.scroll, 0);

        hb.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(hb.scroll, 1);
        hb.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(hb.scroll, 2);
        hb.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(hb.scroll, 1);
        hb.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(hb.scroll, 0);
        // Saturates at 0.
        hb.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(hb.scroll, 0);
    }

    #[test]
    fn app_input_mode_transitions() {
        use super::app::{App, Identity, InputMode};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".to_string(),
        };
        let mut app = App::new(identity, "ws://localhost".to_string());
        assert_eq!(app.mode, InputMode::Normal);

        app.mode = InputMode::Insert;
        app.input_buffer.push_str("hello");
        app.mode = InputMode::Normal;
        assert_eq!(app.input_buffer, "hello"); // preserved across mode swap
    }

    #[test]
    fn mention_detection_uses_self_display_name() {
        use super::buffers::room_timeline::{RoomKind, RoomTimelineBuffer};

        let rb = RoomTimelineBuffer::new(
            spaze_proto::RoomId::new(),
            "# test".to_string(),
            RoomKind::Standard,
            "andreas".to_string(),
        );
        let needle = format!("@{}", rb.self_display_name).to_lowercase();
        assert!(
            "Hello @andreas, how's it going"
                .to_lowercase()
                .contains(&needle)
        );
        assert!(!"hello world".to_lowercase().contains(&needle));
    }

    #[test]
    fn apply_effect_quit_sets_should_quit_flag() {
        use super::app::{App, Identity};
        use spaze_commands::Effect;
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        assert!(!app.should_quit);
        app.apply_effect(Effect::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn apply_effect_clear_empties_active_room_buffer() {
        use super::app::{App, Identity};
        use super::buffers::Buffer;
        use spaze_commands::Effect;
        use spaze_proto::{DeviceId, Message, MessageBody, MessageId, RoomId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        // Push a couple of messages onto the active room buffer.
        if let Buffer::Room(rb) = &mut app.buffers[0] {
            rb.push_message(Message {
                id: MessageId::new(),
                room_id: RoomId::new(),
                author_id: UserId::new(),
                author_device_id: DeviceId::new(),
                author_display_name: "x".into(),
                created_at_ms: 0,
                edited_at_ms: None,
                deleted_at_ms: None,
                body: MessageBody::Text {
                    content: "a".into(),
                },
            });
            rb.push_message(Message {
                id: MessageId::new(),
                room_id: RoomId::new(),
                author_id: UserId::new(),
                author_device_id: DeviceId::new(),
                author_display_name: "x".into(),
                created_at_ms: 0,
                edited_at_ms: None,
                deleted_at_ms: None,
                body: MessageBody::Text {
                    content: "b".into(),
                },
            });
        }
        app.apply_effect(Effect::ClearActiveBuffer);
        if let Buffer::Room(rb) = &app.buffers[0] {
            assert_eq!(rb.messages.len(), 0);
        } else {
            panic!("expected room buffer");
        }
    }

    #[test]
    fn apply_effect_systemline_appends_local_system_message() {
        use super::app::{App, Identity};
        use super::buffers::Buffer;
        use spaze_commands::Effect;
        use spaze_proto::{DeviceId, MessageBody, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        app.apply_effect(Effect::SystemLine("usage: /me <text>".into()));

        if let Buffer::Room(rb) = &app.buffers[0] {
            assert_eq!(rb.messages.len(), 1);
            match &rb.messages[0].body {
                MessageBody::System { content } => {
                    assert_eq!(content, "usage: /me <text>");
                }
                other => panic!("expected System body, got {other:?}"),
            }
        } else {
            panic!("expected room buffer");
        }
    }

    #[test]
    fn body_text_extracts_content_from_action_variant() {
        use super::buffers::room_timeline::body_text_for_test;
        use spaze_proto::MessageBody;

        let body = MessageBody::Action {
            content: "kicks the build".into(),
        };
        assert_eq!(body_text_for_test(&body), "kicks the build");
    }

    #[test]
    fn mention_inside_action_body_is_detectable() {
        // Indirect test: body_text returns content for Action, so the existing
        // mention detection (which lowercases body_text + contains-check)
        // works for actions without further changes.
        use super::buffers::room_timeline::body_text_for_test;
        use spaze_proto::MessageBody;

        let body = MessageBody::Action {
            content: "waves at @beth".into(),
        };
        let needle = "@beth".to_lowercase();
        assert!(body_text_for_test(&body).to_lowercase().contains(&needle));
    }

    #[test]
    fn classify_input_picks_correct_color_for_each_class() {
        use spaze_commands::{InputClass, REGISTRY, classify_input};

        // Validates that each variant has a distinct mapping. The actual
        // theme-color picking lives in tui.rs, but we test the classifier
        // outputs here so a regression in classify_input fails closer to
        // the source.
        assert_eq!(classify_input("hello", REGISTRY), InputClass::Text);
        assert_eq!(classify_input("//me", REGISTRY), InputClass::EscapedText);
        assert!(matches!(
            classify_input("/quit", REGISTRY),
            InputClass::ValidCommand { name: "quit" }
        ));
        assert!(matches!(
            classify_input("/foo", REGISTRY),
            InputClass::InvalidCommand { name: "foo" }
        ));
    }

    #[test]
    fn cursor_width_handles_multibyte_and_wide_chars() {
        use unicode_width::UnicodeWidthStr;
        // ASCII: width = byte len.
        assert_eq!(UnicodeWidthStr::width("abc"), 3);
        // Multi-byte but single-cell (Latin-1 supplement).
        assert_eq!(UnicodeWidthStr::width("ä"), 1);
        assert_eq!(UnicodeWidthStr::width("åäö"), 3);
        // Wide single-codepoint emoji: 2 cells.
        assert_eq!(UnicodeWidthStr::width("😀"), 2);
        // Mixed: ä(1) + 😀(2) + x(1) = 4 cells (but byte len is 2+4+1=7).
        assert_eq!(UnicodeWidthStr::width("ä😀x"), 4);
        assert_eq!("ä😀x".len(), 7); // byte len for comparison
    }

    #[test]
    fn render_action_message_does_not_panic_and_uses_action_path() {
        // Smoke-level render check: rendering an Action message goes through
        // the Action arm (not the wildcard debug path) and produces non-empty
        // output. We can't easily assert visual output in unit tests without
        // a snapshot harness; this asserts the code path is wired up.
        use super::buffers::room_timeline::{RoomKind, RoomTimelineBuffer};
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;
        use spaze_proto::{DeviceId, Message, MessageBody, MessageId, RoomId, UserId};

        let mut rb = RoomTimelineBuffer::new(
            RoomId::new(),
            "# test".into(),
            RoomKind::Standard,
            "self".into(),
        );
        rb.push_message(Message {
            id: MessageId::new(),
            room_id: rb.room_id,
            author_id: UserId::new(),
            author_device_id: DeviceId::new(),
            author_display_name: "andreas".into(),
            created_at_ms: 0,
            edited_at_ms: None,
            deleted_at_ms: None,
            body: MessageBody::Action {
                content: "kicks the build".into(),
            },
        });

        let backend = TestBackend::new(80, 5);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|f| {
                let theme = super::theme::catppuccin_mocha::CATPPUCCIN_MOCHA;
                rb.render(f, f.area(), &theme);
            })
            .expect("draw should not panic");
        // Spot-check that the rendered backend contains "andreas" and "kicks".
        let buf = terminal.backend().buffer().clone();
        let dump: String = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .map(|(x, y)| buf[(x, y)].symbol().to_string())
            .collect();
        assert!(dump.contains("andreas"), "rendered output: {dump}");
        assert!(dump.contains("kicks"), "rendered output: {dump}");
    }

    #[test]
    fn app_initial_focus_is_sidebar_when_sidebar_visible() {
        use super::app::{App, FocusedRegion, Identity};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let app = App::new(identity, "ws://localhost".into());
        assert!(app.sidebar_visible);
        assert_eq!(app.focus, FocusedRegion::Sidebar);
    }

    #[test]
    fn app_has_sidebar_selected_field_initialized_to_first_room() {
        use super::app::{App, Identity};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let app = App::new(identity, "ws://localhost".into());
        // The first Room buffer is at index 0 (Help is at index 1).
        assert_eq!(app.sidebar_selected, Some(0));
    }

    #[test]
    fn clear_command_empties_active_buffer_only() {
        use super::app::{App, Identity};
        use super::buffers::Buffer;
        use spaze_commands::Effect;
        use spaze_proto::{DeviceId, Message, MessageBody, MessageId, RoomId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());

        // Push messages onto the room buffer.
        if let Buffer::Room(rb) = &mut app.buffers[0] {
            for content in ["a", "b", "c"] {
                rb.push_message(Message {
                    id: MessageId::new(),
                    room_id: RoomId::new(),
                    author_id: UserId::new(),
                    author_device_id: DeviceId::new(),
                    author_display_name: "x".into(),
                    created_at_ms: 0,
                    edited_at_ms: None,
                    deleted_at_ms: None,
                    body: MessageBody::Text {
                        content: content.into(),
                    },
                });
            }
        }
        // Apply ClearActiveBuffer.
        app.apply_effect(Effect::ClearActiveBuffer);
        if let Buffer::Room(rb) = &app.buffers[0] {
            assert_eq!(rb.messages.len(), 0, "active buffer should be empty");
        }
    }

    #[test]
    fn quit_command_sets_should_quit_through_handler_chain() {
        use super::app::{App, Identity};
        use spaze_commands::{HandlerContext, REGISTRY, lookup};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        assert!(!app.should_quit);

        let cmd = lookup("quit", REGISTRY).expect("/quit must be in registry");
        let ctx = HandlerContext { registry: REGISTRY };
        let effects = (cmd.handler)(&[], &ctx);
        for effect in effects {
            app.apply_effect(effect);
        }
        assert!(app.should_quit);
    }

    #[test]
    fn set_focus_input_enters_insert_mode() {
        use super::app::{App, FocusedRegion, Identity, InputMode};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        assert_eq!(app.mode, InputMode::Normal);
        app.set_focus(FocusedRegion::Input);
        assert_eq!(app.focus, FocusedRegion::Input);
        assert_eq!(app.mode, InputMode::Insert);
    }

    #[test]
    fn set_focus_non_input_exits_insert_mode() {
        use super::app::{App, FocusedRegion, Identity, InputMode};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        app.mode = InputMode::Insert;
        app.focus = FocusedRegion::Input;
        app.set_focus(FocusedRegion::Sidebar);
        assert_eq!(app.focus, FocusedRegion::Sidebar);
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn set_focus_non_input_from_normal_does_not_change_mode() {
        use super::app::{App, FocusedRegion, Identity, InputMode};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        assert_eq!(app.mode, InputMode::Normal);
        app.set_focus(FocusedRegion::Buffer);
        assert_eq!(app.focus, FocusedRegion::Buffer);
        assert_eq!(app.mode, InputMode::Normal);
    }
}
