//! End-to-end integration test for Phase 1.A.
//!
//! Spawns the server in-process on an OS-assigned port, then opens two raw
//! WebSocket connections and asserts the message flow:
//!
//! 1. Client A sends `PostMessage("hello")` -> A receives Response, B receives Event.
//! 2. Client B sends `PostMessage("hi back")` -> B receives Response, A receives Event.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use futures_util::{SinkExt, StreamExt};
use spaze_client::identity::derive_identity;
use spaze_proto::{
    ClientCommand, ClientFrame, CommandOutcome, MessageBody, RequestId, ResponsePayload, RoomId,
    ServerEvent, ServerFrame,
};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use uuid::Uuid;

const STEP_TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::too_many_lines)] // Linear narrative test; splitting hurts readability.
async fn two_clients_can_chat() -> Result<()> {
    // 1. Start the server on an OS-assigned port.
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .context("bind")?;
    let actual_addr = listener.local_addr()?;
    let server_url = format!("ws://{actual_addr}/");

    // We can't use spaze_server::run directly because it owns the listener
    // creation. Replicate the server loop body here for the test.
    let state = spaze_server::ServerState::new();
    let server_handle = {
        let state = state.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, peer)) = listener.accept().await else {
                    return;
                };
                let conn_id = spaze_server::ConnectionId(
                    state
                        .next_connection_id
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                );
                let s = state.clone();
                tokio::spawn(async move {
                    spaze_server::connection::handle_connection(stream, s, conn_id, peer).await;
                });
            }
        })
    };

    // 2. Connect two clients.
    let (mut a_ws, _) = tokio_tungstenite::connect_async(&server_url)
        .await
        .context("client A connect")?;
    let (mut b_ws, _) = tokio_tungstenite::connect_async(&server_url)
        .await
        .context("client B connect")?;

    let (a_user, a_device) = derive_identity("andreas");
    let (b_user, b_device) = derive_identity("beth");
    let room = RoomId::from_uuid(Uuid::nil());

    // 3. A sends PostMessage("hello").
    let frame_a = ClientFrame {
        request_id: RequestId(1),
        command: ClientCommand::PostMessage {
            room_id: room,
            author_id: a_user,
            author_device_id: a_device,
            author_display_name: "andreas".to_string(),
            body: MessageBody::Text {
                content: "hello".to_string(),
            },
        },
    };
    a_ws.send(WsMessage::Text(serde_json::to_string(&frame_a)?))
        .await?;

    // A should receive a Response with the canonical Message.
    let a_response = next_server_frame(&mut a_ws).await?;
    match a_response {
        ServerFrame::Response { request_id, result } => {
            assert_eq!(request_id, RequestId(1));
            match result {
                CommandOutcome::Ok {
                    payload: ResponsePayload::MessagePosted(msg),
                } => {
                    assert_eq!(msg.author_id, a_user);
                    if let MessageBody::Text { content } = &msg.body {
                        assert_eq!(content, "hello");
                    } else {
                        return Err(anyhow!("body should be Text"));
                    }
                }
                other => return Err(anyhow!("expected Ok(MessagePosted), got {other:?}")),
            }
        }
        other @ ServerFrame::Event(_) => {
            return Err(anyhow!("expected Response, got {other:?}"));
        }
    }

    // B should receive an Event with the same message.
    let b_event = next_server_frame(&mut b_ws).await?;
    match b_event {
        ServerFrame::Event(ServerEvent::MessagePosted(msg)) => {
            assert_eq!(msg.author_id, a_user);
            if let MessageBody::Text { content } = &msg.body {
                assert_eq!(content, "hello");
            } else {
                return Err(anyhow!("body should be Text"));
            }
        }
        other => return Err(anyhow!("expected Event(MessagePosted), got {other:?}")),
    }

    // 4. B sends PostMessage("hi back").
    let frame_b = ClientFrame {
        request_id: RequestId(1),
        command: ClientCommand::PostMessage {
            room_id: room,
            author_id: b_user,
            author_device_id: b_device,
            author_display_name: "beth".to_string(),
            body: MessageBody::Text {
                content: "hi back".to_string(),
            },
        },
    };
    b_ws.send(WsMessage::Text(serde_json::to_string(&frame_b)?))
        .await?;

    // B receives Response.
    let b_response = next_server_frame(&mut b_ws).await?;
    match b_response {
        ServerFrame::Response {
            result:
                CommandOutcome::Ok {
                    payload: ResponsePayload::MessagePosted(msg),
                },
            ..
        } => {
            assert_eq!(msg.author_id, b_user);
        }
        other => return Err(anyhow!("expected Ok(MessagePosted) on B, got {other:?}")),
    }

    // A receives Event.
    let a_event = next_server_frame(&mut a_ws).await?;
    match a_event {
        ServerFrame::Event(ServerEvent::MessagePosted(msg)) => {
            assert_eq!(msg.author_id, b_user);
            if let MessageBody::Text { content } = &msg.body {
                assert_eq!(content, "hi back");
            }
        }
        other => return Err(anyhow!("expected Event on A, got {other:?}")),
    }

    // 5. Clean shutdown.
    a_ws.send(WsMessage::Close(None)).await?;
    b_ws.send(WsMessage::Close(None)).await?;
    server_handle.abort();

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_json_returns_invalid_request_without_dropping_connection() -> Result<()> {
    let listener =
        tokio::net::TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).await?;
    let actual_addr = listener.local_addr()?;
    let server_url = format!("ws://{actual_addr}/");

    let state = spaze_server::ServerState::new();
    let server_handle = {
        let state = state.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, peer)) = listener.accept().await else {
                    return;
                };
                let conn_id = spaze_server::ConnectionId(
                    state
                        .next_connection_id
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                );
                let s = state.clone();
                tokio::spawn(async move {
                    spaze_server::connection::handle_connection(stream, s, conn_id, peer).await;
                });
            }
        })
    };

    let (mut ws, _) = tokio_tungstenite::connect_async(&server_url).await?;

    // Send garbage.
    ws.send(WsMessage::Text("not json".into())).await?;

    let response = next_server_frame(&mut ws).await?;
    match response {
        ServerFrame::Response {
            result: CommandOutcome::Err { error },
            ..
        } => {
            // Should be InvalidRequest. Specific message text is implementation detail.
            assert!(
                matches!(error, spaze_proto::ProtocolError::InvalidRequest { .. }),
                "expected InvalidRequest, got {error:?}"
            );
        }
        other => return Err(anyhow!("expected Err response, got {other:?}")),
    }

    // Connection should still be alive — send a valid frame and expect a response.
    let frame = ClientFrame {
        request_id: RequestId(2),
        command: ClientCommand::PostMessage {
            room_id: RoomId::from_uuid(Uuid::nil()),
            author_id: derive_identity("test").0,
            author_device_id: derive_identity("test").1,
            author_display_name: "test".to_string(),
            body: MessageBody::Text {
                content: "still here".to_string(),
            },
        },
    };
    ws.send(WsMessage::Text(serde_json::to_string(&frame)?))
        .await?;
    let response = next_server_frame(&mut ws).await?;
    assert!(
        matches!(
            response,
            ServerFrame::Response {
                result: CommandOutcome::Ok { .. },
                ..
            }
        ),
        "connection should still process valid frames after bad JSON; got {response:?}"
    );

    ws.send(WsMessage::Close(None)).await?;
    server_handle.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn me_action_message_roundtrips_between_two_clients() -> Result<()> {
    // 1. Spawn the server (same pattern as `two_clients_can_chat`).
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let listener = tokio::net::TcpListener::bind(bind_addr).await.context("bind")?;
    let actual_addr = listener.local_addr()?;
    let server_url = format!("ws://{actual_addr}/");

    let state = spaze_server::ServerState::new();
    let _server_handle = {
        let state = state.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, peer)) = listener.accept().await else {
                    return;
                };
                let conn_id = spaze_server::ConnectionId(
                    state
                        .next_connection_id
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                );
                let s = state.clone();
                tokio::spawn(async move {
                    spaze_server::connection::handle_connection(stream, s, conn_id, peer).await;
                });
            }
        })
    };

    // 2. Connect two clients.
    let (a_user, a_device) = derive_identity("andreas");
    let (mut a_ws, _) = tokio_tungstenite::connect_async(&server_url)
        .await
        .context("a connect")?;
    let (mut b_ws, _) = tokio_tungstenite::connect_async(&server_url)
        .await
        .context("b connect")?;

    // Tiny delay so both registrations land before sending.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 3. A sends an Action message via PostMessage.
    let room_id = RoomId::from_uuid(Uuid::nil());
    let frame = ClientFrame {
        request_id: RequestId(1),
        command: ClientCommand::PostMessage {
            room_id,
            author_id: a_user,
            author_device_id: a_device,
            author_display_name: "andreas".into(),
            body: MessageBody::Action {
                content: "kicks the build".into(),
            },
        },
    };
    let json = serde_json::to_string(&frame).context("serialize")?;
    a_ws.send(WsMessage::Text(json)).await.context("a send")?;

    // 4. B should receive a ServerFrame::Event with Action body.
    let server_frame = next_server_frame(&mut b_ws).await?;
    match server_frame {
        ServerFrame::Event(ServerEvent::MessagePosted(m)) => match m.body {
            MessageBody::Action { content } => {
                assert_eq!(content, "kicks the build");
            }
            other => return Err(anyhow!("expected Action body, got {other:?}")),
        },
        other => return Err(anyhow!("expected Event(MessagePosted), got {other:?}")),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn me_with_utf8_content_roundtrips_intact() -> Result<()> {
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let listener = tokio::net::TcpListener::bind(bind_addr).await.context("bind")?;
    let actual_addr = listener.local_addr()?;
    let server_url = format!("ws://{actual_addr}/");

    let state = spaze_server::ServerState::new();
    let _server_handle = {
        let state = state.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, peer)) = listener.accept().await else {
                    return;
                };
                let conn_id = spaze_server::ConnectionId(
                    state
                        .next_connection_id
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                );
                let s = state.clone();
                tokio::spawn(async move {
                    spaze_server::connection::handle_connection(stream, s, conn_id, peer).await;
                });
            }
        })
    };

    let (a_user, a_device) = derive_identity("andreas");
    let (mut a_ws, _) = tokio_tungstenite::connect_async(&server_url)
        .await
        .context("a connect")?;
    let (mut b_ws, _) = tokio_tungstenite::connect_async(&server_url)
        .await
        .context("b connect")?;

    tokio::time::sleep(Duration::from_millis(50)).await;

    let frame = ClientFrame {
        request_id: RequestId(1),
        command: ClientCommand::PostMessage {
            room_id: RoomId::from_uuid(Uuid::nil()),
            author_id: a_user,
            author_device_id: a_device,
            author_display_name: "andreas".into(),
            body: MessageBody::Action {
                content: "waves at åse 🐧".into(),
            },
        },
    };
    a_ws.send(WsMessage::Text(serde_json::to_string(&frame)?))
        .await
        .context("a send")?;

    let server_frame = next_server_frame(&mut b_ws).await?;
    let ServerFrame::Event(ServerEvent::MessagePosted(m)) = server_frame else {
        return Err(anyhow!("expected Event MessagePosted"));
    };
    let MessageBody::Action { content } = m.body else {
        return Err(anyhow!("expected Action body"));
    };
    assert_eq!(content, "waves at åse 🐧");
    Ok(())
}

/// Read the next text frame from a WS connection and parse it as `ServerFrame`.
async fn next_server_frame(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Result<ServerFrame> {
    let next = timeout(STEP_TIMEOUT, ws.next())
        .await
        .map_err(|_| anyhow!("timed out waiting for server frame"))?
        .ok_or_else(|| anyhow!("ws stream ended"))?
        .context("ws read error")?;
    match next {
        WsMessage::Text(text) => {
            let frame: ServerFrame = serde_json::from_str(&text).context("parse ServerFrame")?;
            Ok(frame)
        }
        other => Err(anyhow!("unexpected WS message type: {other:?}")),
    }
}
