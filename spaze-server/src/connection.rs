//! Per-connection task. Real body lands in Task 5.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpStream;
use tracing::info;

use crate::{ConnectionId, ServerState};

/// Handle a single accepted TCP connection. Stub for now.
#[allow(clippy::unused_async)] // body (with awaits) lands in Task 5; signature is stable.
pub async fn handle_connection(
    _stream: TcpStream,
    _state: Arc<ServerState>,
    conn_id: ConnectionId,
    peer: SocketAddr,
) {
    info!(
        "connection {} from {} accepted (stub — closing immediately)",
        conn_id.0, peer
    );
    // Real implementation in Task 5.
}
