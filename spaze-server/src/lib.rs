//! `spaze-server` — daemon for the Spaze chat protocol.
//!
//! Phase 1.A scope: single hardcoded Room, plaintext WebSocket, no auth.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use anyhow::{Context, Result};
use spaze_proto::{RoomId, ServerEvent};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{info, warn};

pub mod connection;
pub mod time;

/// CLI-level configuration for `run`.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
}

/// Shared per-server state passed to each connection task.
pub struct ServerState {
    /// Single broadcast channel — there is one Room in 1.A.
    /// Each event carries the sender's `ConnectionId` so subscribers can skip self.
    pub broadcast: broadcast::Sender<(ConnectionId, ServerEvent)>,
    /// The hardcoded Room ID for 1.A.
    pub room_id: RoomId,
    /// Allocator for opaque per-connection IDs. Not authentication.
    pub next_connection_id: AtomicU64,
}

/// Opaque per-connection identity. Used only to skip the sender on broadcast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionId(pub u64);

impl ServerState {
    /// Construct shared state. Broadcast buffer of 64 — see phase-1a spec.
    #[must_use]
    pub fn new() -> Arc<Self> {
        let (broadcast, _rx) = broadcast::channel(64);
        Arc::new(Self {
            broadcast,
            room_id: RoomId::new(),
            next_connection_id: AtomicU64::new(0),
        })
    }
}

/// Run the server until Ctrl+C.
///
/// # Errors
///
/// Returns an error if the listener cannot bind to `config.bind_addr`.
pub async fn run(config: ServerConfig) -> Result<()> {
    let state = ServerState::new();

    let listener = TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.bind_addr))?;
    let actual_addr = listener.local_addr()?;
    info!("listening on {actual_addr}");

    loop {
        tokio::select! {
            accept = listener.accept() => {
                match accept {
                    Ok((stream, peer)) => {
                        let conn_id = ConnectionId(
                            state
                                .next_connection_id
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        );
                        let state = Arc::clone(&state);
                        tokio::spawn(async move {
                            connection::handle_connection(stream, state, conn_id, peer).await;
                        });
                    }
                    Err(err) => {
                        warn!("accept failed: {err}");
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("shutting down");
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_unix_ms_is_positive_and_advances() {
        let a = time::now_unix_ms();
        let b = time::now_unix_ms();
        assert!(a > 1_700_000_000_000, "timestamp should be after Nov 2023");
        assert!(b >= a, "time should be non-decreasing");
    }

    #[test]
    fn server_state_initializes_with_distinct_connection_ids() {
        let state = ServerState::new();
        let a = state
            .next_connection_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let b = state
            .next_connection_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(a, 0);
        assert_eq!(b, 1);
    }
}
