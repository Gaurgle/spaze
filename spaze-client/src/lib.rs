//! `spaze-client` — TUI client for the Spaze chat protocol.
//!
//! Phase 1.A scope: stdin/stdout chat, single hardcoded Room.

use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use spaze_proto::{
    ClientCommand, ClientFrame, DeviceId, MessageBody, RequestId, RoomId, ServerFrame, UserId,
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{info, warn};

pub mod identity;
pub mod render;

/// Configuration for the client connection.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub server_url: String,
    pub user_id: UserId,
    pub device_id: DeviceId,
    pub room_id: RoomId,
}

/// Run the client until stdin closes, server disconnects, or Ctrl+C.
///
/// Reads lines from stdin, sends each as a `PostMessage`, and renders incoming
/// frames via `render::render_frame`.
///
/// # Errors
///
/// Returns an error on connection failure or unrecoverable WS error.
pub async fn run(config: ClientConfig) -> Result<()> {
    let (ws, _resp) = tokio_tungstenite::connect_async(&config.server_url)
        .await
        .with_context(|| format!("failed to connect to {}", config.server_url))?;
    info!("connected to {}", config.server_url);

    let (mut sink, mut stream) = ws.split();
    let mut stdin = BufReader::new(tokio::io::stdin()).lines();
    let request_counter = AtomicU64::new(1);

    loop {
        tokio::select! {
            line = stdin.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        let frame = ClientFrame {
                            request_id: RequestId(
                                request_counter.fetch_add(1, Ordering::Relaxed),
                            ),
                            command: ClientCommand::PostMessage {
                                room_id: config.room_id,
                                author_id: config.user_id,
                                author_device_id: config.device_id,
                                author_display_name: String::new(), // throwaway — Task 10 replaces this file
                                body: MessageBody::Text { content: line },
                            },
                        };
                        let json = serde_json::to_string(&frame)
                            .context("serialize ClientFrame")?;
                        sink.send(WsMessage::Text(json))
                            .await
                            .context("ws sink write")?;
                    }
                    Ok(None) => {
                        info!("stdin closed");
                        let _ = sink.send(WsMessage::Close(None)).await;
                        break;
                    }
                    Err(err) => {
                        warn!("stdin read error: {err}");
                        break;
                    }
                }
            }
            incoming = stream.next() => {
                match incoming {
                    Some(Ok(WsMessage::Text(text))) => {
                        match serde_json::from_str::<ServerFrame>(&text) {
                            Ok(frame) => render::render_frame(&frame),
                            Err(err) => warn!("could not parse ServerFrame: {err}"),
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        eprintln!("connection lost: server closed");
                        std::process::exit(1);
                    }
                    Some(Ok(_)) => {} // ping/pong handled by tungstenite; binary unused
                    Some(Err(err)) => {
                        eprintln!("connection lost: {err}");
                        std::process::exit(1);
                    }
                    None => {
                        eprintln!("connection lost: stream ended");
                        std::process::exit(1);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("ctrl+c — shutting down");
                let _ = sink.send(WsMessage::Close(None)).await;
                break;
            }
        }
    }

    Ok(())
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
}
