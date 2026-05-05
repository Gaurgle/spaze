//! Per-connection task: handshake, command dispatch, broadcast subscription.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use spaze_proto::{
    ClientCommand, ClientFrame, CommandOutcome, Message, MessageId, ProtocolError, ResponsePayload,
    ServerEvent, ServerFrame,
};
use tokio::net::TcpStream;
use tokio::sync::broadcast::error::RecvError;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{info, warn};

use crate::time::now_unix_ms;
use crate::{ConnectionId, ServerState};

/// Handle a single accepted TCP connection from start to finish.
pub async fn handle_connection(
    stream: TcpStream,
    state: Arc<ServerState>,
    conn_id: ConnectionId,
    peer: SocketAddr,
) {
    let ws = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(err) => {
            warn!("ws handshake failed for {peer}: {err}");
            return;
        }
    };
    info!("connection {} from {peer} established", conn_id.0);

    let (mut sink, mut ws_stream) = ws.split();
    let mut rx = state.broadcast.subscribe();

    loop {
        tokio::select! {
            incoming = ws_stream.next() => {
                match incoming {
                    Some(Ok(WsMessage::Text(text))) => {
                        if let Err(err) = handle_text_frame(text.as_str(), &state, conn_id, &mut sink).await {
                            warn!("handle_text_frame error for connection {}: {err:#}", conn_id.0);
                            break;
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        info!("connection {} sent close frame", conn_id.0);
                        break;
                    }
                    Some(Ok(WsMessage::Binary(_) | WsMessage::Ping(_) | WsMessage::Pong(_) | WsMessage::Frame(_))) => {
                        // tungstenite handles ping/pong itself; binary not supported in 1.A.
                    }
                    Some(Err(err)) => {
                        info!("connection {} ws error: {err}", conn_id.0);
                        break;
                    }
                    None => {
                        info!("connection {} ws stream ended", conn_id.0);
                        break;
                    }
                }
            }
            broadcast = rx.recv() => {
                match broadcast {
                    Ok((sender_id, event)) if sender_id != conn_id => {
                        let frame = ServerFrame::Event(event);
                        let json = match serde_json::to_string(&frame) {
                            Ok(j) => j,
                            Err(err) => {
                                warn!("serialize broadcast event failed: {err}");
                                continue;
                            }
                        };
                        if let Err(err) = sink.send(WsMessage::Text(json)).await {
                            info!("connection {} sink write failed: {err}", conn_id.0);
                            break;
                        }
                    }
                    Ok(_self_event) => {
                        // Skip — sender shouldn't receive their own broadcast (Option B).
                    }
                    Err(RecvError::Lagged(n)) => {
                        warn!("connection {} lagged behind broadcast by {n} events", conn_id.0);
                    }
                    Err(RecvError::Closed) => {
                        info!("broadcast closed; connection {} exiting", conn_id.0);
                        break;
                    }
                }
            }
        }
    }

    info!("connection {} closed", conn_id.0);
}

/// Parse a text frame as `ClientFrame` and dispatch.
async fn handle_text_frame(
    text: &str,
    state: &ServerState,
    conn_id: ConnectionId,
    sink: &mut WsSink,
) -> Result<()> {
    let frame: ClientFrame = match serde_json::from_str(text) {
        Ok(f) => f,
        Err(err) => {
            // Malformed JSON or schema mismatch. Respond with InvalidRequest using request_id 0
            // (we don't know the real one). Connection stays alive.
            let response = ServerFrame::Response {
                request_id: spaze_proto::RequestId(0),
                result: CommandOutcome::Err {
                    error: ProtocolError::InvalidRequest {
                        message: format!("could not parse ClientFrame: {err}"),
                    },
                },
            };
            send_frame(sink, &response).await?;
            return Ok(());
        }
    };

    handle_command(frame, state, conn_id, sink).await
}

/// Dispatch a parsed `ClientFrame`. Sends the appropriate response and
/// (for `PostMessage`) broadcasts an event to other subscribers.
// `match` (not `if let`) is intentional: Edit/Delete/Join/Leave/Typing land in Phase 2
// and will become explicit arms here.
#[allow(clippy::single_match_else)]
async fn handle_command(
    frame: ClientFrame,
    state: &ServerState,
    conn_id: ConnectionId,
    sink: &mut WsSink,
) -> Result<()> {
    match frame.command {
        ClientCommand::PostMessage {
            room_id,
            author_id,
            author_device_id,
            author_display_name,
            body,
        } => {
            let msg = Message {
                id: MessageId::new(),
                room_id,
                author_id,
                author_device_id,
                author_display_name,
                created_at_ms: now_unix_ms(),
                edited_at_ms: None,
                deleted_at_ms: None,
                body,
            };

            // Response to sender: canonical Message.
            let response = ServerFrame::Response {
                request_id: frame.request_id,
                result: CommandOutcome::Ok {
                    payload: ResponsePayload::MessagePosted(msg.clone()),
                },
            };
            send_frame(sink, &response).await?;

            // Broadcast to others (subscribers skip self via ConnectionId).
            // SendError = no subscribers; harmless.
            let _ = state
                .broadcast
                .send((conn_id, ServerEvent::MessagePosted(msg)));
        }

        // Edit/Delete/Join/Leave/Typing: deferred to Phase 2.
        _ => {
            let response = ServerFrame::Response {
                request_id: frame.request_id,
                result: CommandOutcome::Err {
                    error: ProtocolError::InvalidRequest {
                        message: "command not implemented in Phase 1.A".to_string(),
                    },
                },
            };
            send_frame(sink, &response).await?;
        }
    }
    Ok(())
}

async fn send_frame(sink: &mut WsSink, frame: &ServerFrame) -> Result<()> {
    let json = serde_json::to_string(frame).context("serialize ServerFrame")?;
    sink.send(WsMessage::Text(json))
        .await
        .context("ws sink write")?;
    Ok(())
}

// Type alias for the split sink half. Saves on long generic bounds elsewhere.
type WsSink =
    futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, WsMessage>;
