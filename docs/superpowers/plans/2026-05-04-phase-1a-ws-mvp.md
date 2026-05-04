# Spaze Phase 1.A — WS Round-Trip MVP — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a bare WS chat — two `spaze` clients connect to one `spaze-server` over plaintext WebSocket and exchange messages via stdin/stdout. Single hardcoded Room. No TUI yet.

**Architecture:** Tokio + tokio-tungstenite throughout. Server holds a `tokio::sync::broadcast::Sender<(ConnectionId, ServerEvent)>` and spawns one task per connection. Each connection task uses `tokio::select!` over incoming WS frames + broadcast events. Client mirrors the shape: `select!` over stdin lines + WS frames + Ctrl+C. Identity is derived deterministically from `--name` via UUIDv5. Wire: JSON over WS, using the proto types we shipped at v0.1.0 (bumped to v0.2.0 by this plan to add `author_id` / `author_device_id` to `PostMessage`).

**Tech Stack:** Rust 2024 edition, tokio, tokio-tungstenite, futures-util, serde + serde_json (already in proto), tracing + tracing-subscriber, clap (derive), anyhow, uuid (v5 + v7).

**Spec reference:** `docs/superpowers/specs/2026-05-04-phase-1a-ws-mvp-design.md` (canonical).

**Branching:** All tasks in this plan land on a new feature branch `phase-1a`. PR to `main` happens at the end (Task 12). Per Andreas's CLAUDE.md, all commits should be presented for review; subagent-driven execution may proceed task-by-task with the user's blanket authorization given before execution starts.

**Conventions:**
- Conventional Commits (`feat:`, `chore:`, `test:`, `refactor:`, `docs:`).
- First line ≤72 chars. Body optional.
- **No `Co-Authored-By` lines, ever** (per Andreas's CLAUDE.md).

---

## File Structure

This plan creates and modifies these files. Two existing files are touched (root `Cargo.toml`, `spaze-proto/src/events.rs` + tests in `spaze-proto/src/lib.rs`); everything else is new.

```
Spaze/
├── Cargo.toml                               # MODIFY — version bump, new workspace deps
├── spaze-proto/
│   └── src/
│       ├── events.rs                        # MODIFY — PostMessage gains author fields
│       └── lib.rs                           # MODIFY — update existing tests, add new test
├── spaze-server/
│   ├── Cargo.toml                           # MODIFY — add deps, dev-deps for integration test
│   ├── src/
│   │   ├── main.rs                          # MODIFY — thin CLI wrapper over lib
│   │   ├── lib.rs                           # CREATE — ServerState, run(), public surface
│   │   ├── connection.rs                    # CREATE — handle_connection + dispatch
│   │   └── time.rs                          # CREATE — now_unix_ms()
│   └── tests/
│       └── two_client_chat.rs               # CREATE — integration test
└── spaze-client/
    ├── Cargo.toml                           # MODIFY — add deps
    └── src/
        ├── main.rs                          # MODIFY — thin CLI wrapper over lib
        ├── lib.rs                           # CREATE — public surface, connect()
        ├── identity.rs                      # CREATE — derive_identity() + namespace UUID
        └── render.rs                        # CREATE — print_message + frame dispatch
```

Responsibility split:
- **`spaze-server/src/lib.rs`** — public API (`run(ServerConfig) -> Result<()>`), `ServerConfig`, `ServerState`, `ConnectionId`. The binary in `main.rs` is a thin clap wrapper that constructs `ServerConfig` and calls `run()`.
- **`spaze-server/src/connection.rs`** — `handle_connection` task body and `handle_command` dispatch logic. Imports types from `lib.rs`.
- **`spaze-server/src/time.rs`** — single helper `now_unix_ms() -> i64`. Trivial but testable; lives in its own file so we don't sprinkle `SystemTime` plumbing through the server.
- **`spaze-server/tests/two_client_chat.rs`** — integration test. Depends on `spaze-server` (the lib) and `spaze-client` (dev-dep) so it can drive both ends in-process.
- **`spaze-client/src/lib.rs`** — public API (`connect(ClientConfig) -> Result<...>`, structured to be drivable from tests as well as main).
- **`spaze-client/src/identity.rs`** — `derive_identity(name) -> (UserId, DeviceId)` + the namespace UUID constant.
- **`spaze-client/src/render.rs`** — `print_message(&Message)` and `render_frame(ServerFrame)` — kept separate so the integration test can call them or replace them with assertions.
- **`spaze-client/src/main.rs`** — thin clap wrapper that resolves `--name`, calls `lib::connect()`, and runs the stdin loop.

---

## Task 1: Create branch, bump workspace version, add new dependencies

**Files:**
- Create branch: `phase-1a` (off `main`)
- Modify: `/Users/at-a/Repos/Spaze/Cargo.toml`

- [ ] **Step 1: Create the feature branch**

Run from `/Users/at-a/Repos/Spaze`:

```bash
git checkout -b phase-1a
```

Expected: `Switched to a new branch 'phase-1a'`. Verify with `git branch --show-current` → `phase-1a`.

- [ ] **Step 2: Bump workspace version and add new dependencies**

Modify the root `Cargo.toml`. Find this block:

```toml
[workspace.package]
version = "0.1.0"
```

Change to:

```toml
[workspace.package]
version = "0.2.0"
```

Then find the `[workspace.dependencies]` block and add the new deps below the existing ones. The full updated block should be:

```toml
[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
uuid = { version = "1.10", features = ["v7", "v5", "serde"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "io-std", "signal", "net", "sync", "time"] }
tokio-tungstenite = "0.24"
futures-util = { version = "0.3", default-features = false, features = ["sink"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
clap = { version = "4", features = ["derive"] }
anyhow = "1"
```

Note: `uuid` features now include `"v5"` (previously just `"v7"` and `"serde"`). UUIDv5 is needed for client identity derivation in Task 6.

- [ ] **Step 3: Verify workspace still compiles**

Run: `cd /Users/at-a/Repos/Spaze && cargo check --workspace`
Expected: clean build. The new deps are added but not yet *used* — this just verifies the manifest parses and the lockfile resolves.

If `cargo check` fails with a "no matching versions for x" or feature mismatch, fix the manifest before continuing.

- [ ] **Step 4: Verify formatter is happy**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: both clean.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: bump to 0.2.0 and add tokio/tungstenite/clap deps for phase 1.A"
```

---

## Task 2: Proto change — `PostMessage` gains author fields (TDD)

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/spaze-proto/src/events.rs`
- Modify: `/Users/at-a/Repos/Spaze/spaze-proto/src/lib.rs` (update existing tests + add new test)

- [ ] **Step 1: Write the new failing test**

Add to the `tests` module in `spaze-proto/src/lib.rs` (immediately after `client_frame_carries_request_id_and_command`):

```rust
#[test]
fn post_message_carries_author_fields() {
    let user = UserId::new();
    let device = DeviceId::new();
    let cmd = ClientCommand::PostMessage {
        room_id: RoomId::new(),
        author_id: user,
        author_device_id: device,
        body: MessageBody::Text { content: "hello".into() },
    };
    let json = serde_json::to_string(&cmd).unwrap();
    assert!(json.contains("\"type\":\"post_message\""));
    assert!(
        json.contains("author_id"),
        "author_id field missing in {json}"
    );
    assert!(
        json.contains("author_device_id"),
        "author_device_id field missing in {json}"
    );
    let parsed: ClientCommand = serde_json::from_str(&json).unwrap();
    match parsed {
        ClientCommand::PostMessage {
            author_id,
            author_device_id,
            ..
        } => {
            assert_eq!(author_id, user);
            assert_eq!(author_device_id, device);
        }
        other => panic!("wrong variant: {other:?}"),
    }
}
```

- [ ] **Step 2: Run the new test — verify it fails**

Run: `cargo test -p spaze-proto post_message_carries_author_fields`
Expected: compile failure — `PostMessage` does not have `author_id` or `author_device_id` fields.

- [ ] **Step 3: Modify `PostMessage` in `events.rs`**

Find the `ClientCommand::PostMessage` variant in `/Users/at-a/Repos/Spaze/spaze-proto/src/events.rs`:

```rust
PostMessage {
    room_id: RoomId,
    body: MessageBody,
},
```

Replace with:

```rust
PostMessage {
    room_id: RoomId,
    author_id: UserId,
    author_device_id: DeviceId,
    body: MessageBody,
},
```

Also verify the `use crate::ids::{...}` line at the top of `events.rs` includes `UserId` and `DeviceId`. If it currently imports only `MessageId, RoomId, UserId`, add `DeviceId`. The full import line should be:

```rust
use crate::ids::{DeviceId, MessageId, RoomId, UserId};
```

- [ ] **Step 4: Update `client_command_roundtrips_json` in `lib.rs`**

The existing test currently has:

```rust
let cmd = ClientCommand::PostMessage {
    room_id: RoomId::new(),
    body: MessageBody::Text { content: "hi".into() },
};
```

Update to:

```rust
let cmd = ClientCommand::PostMessage {
    room_id: RoomId::new(),
    author_id: UserId::new(),
    author_device_id: DeviceId::new(),
    body: MessageBody::Text { content: "hi".into() },
};
```

- [ ] **Step 5: Update `client_frame_carries_request_id_and_command` in `lib.rs`**

The existing test currently has:

```rust
let frame = ClientFrame {
    request_id: RequestId(42),
    command: ClientCommand::PostMessage {
        room_id: RoomId::new(),
        body: MessageBody::Text { content: "hi".into() },
    },
};
```

Update to:

```rust
let frame = ClientFrame {
    request_id: RequestId(42),
    command: ClientCommand::PostMessage {
        room_id: RoomId::new(),
        author_id: UserId::new(),
        author_device_id: DeviceId::new(),
        body: MessageBody::Text { content: "hi".into() },
    },
};
```

- [ ] **Step 6: Run all proto tests**

Run: `cargo test -p spaze-proto`
Expected: 14 tests pass (the existing 13 plus the new `post_message_carries_author_fields`). No failures.

- [ ] **Step 7: Run clippy and fmt**

Run: `cargo clippy -p spaze-proto --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: both clean.

- [ ] **Step 8: Commit**

```bash
git add spaze-proto/src/events.rs spaze-proto/src/lib.rs
git commit -m "feat(proto): add author_id and author_device_id to PostMessage" -m "Required by Phase 1.A — server is identity-agnostic and trusts the client to put author info on each PostMessage. Phase 3 will validate against authenticated session."
```

---

## Task 3: `spaze-server` time helper

**Files:**
- Create: `/Users/at-a/Repos/Spaze/spaze-server/src/time.rs`
- Create: `/Users/at-a/Repos/Spaze/spaze-server/src/lib.rs` (just the module declaration for now)

- [ ] **Step 1: Write the failing test**

Create `/Users/at-a/Repos/Spaze/spaze-server/src/lib.rs` with:

```rust
//! `spaze-server` — daemon for the Spaze chat protocol.

pub mod time;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_unix_ms_is_positive_and_advances() {
        let a = time::now_unix_ms();
        // Sleep is overkill; just call again — millisecond precision is enough.
        let b = time::now_unix_ms();
        assert!(a > 1_700_000_000_000, "timestamp should be after Nov 2023");
        assert!(b >= a, "time should be non-decreasing");
    }
}
```

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p spaze-server`
Expected: compile failure — `time` module does not exist.

- [ ] **Step 3: Implement `spaze-server/src/time.rs`**

```rust
//! Time helpers. Centralized so the rest of the server doesn't sprinkle `SystemTime` plumbing.

use std::time::{SystemTime, UNIX_EPOCH};

/// Current unix timestamp in milliseconds.
///
/// # Panics
///
/// Panics if the system clock is set before the unix epoch (1970-01-01),
/// which would indicate a deeply broken machine.
#[must_use]
pub fn now_unix_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before unix epoch")
            .as_millis(),
    )
    .expect("timestamp does not fit in i64")
}
```

- [ ] **Step 4: Run tests — verify they pass**

Run: `cargo test -p spaze-server`
Expected: 1 test passes.

- [ ] **Step 5: Run clippy + fmt**

Run: `cargo clippy -p spaze-server --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: both clean.

- [ ] **Step 6: Commit**

```bash
git add spaze-server/src/lib.rs spaze-server/src/time.rs
git commit -m "feat(server): add now_unix_ms() time helper"
```

---

## Task 4: `spaze-server` — `ServerState`, `ConnectionId`, `ServerConfig`, `run()` skeleton

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/spaze-server/Cargo.toml`
- Modify: `/Users/at-a/Repos/Spaze/spaze-server/src/lib.rs`
- Modify: `/Users/at-a/Repos/Spaze/spaze-server/src/main.rs`

- [ ] **Step 1: Update `spaze-server/Cargo.toml`**

Replace the existing `[dependencies]` block with:

```toml
[dependencies]
spaze-proto = { path = "../spaze-proto" }
tokio.workspace = true
tokio-tungstenite.workspace = true
futures-util.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
clap.workspace = true
anyhow.workspace = true
serde_json.workspace = true
```

The full file should now look like:

```toml
[package]
name = "spaze-server"
description = "Spaze server daemon"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true

[lints]
workspace = true

[[bin]]
name = "spaze-server"
path = "src/main.rs"

[dependencies]
spaze-proto = { path = "../spaze-proto" }
tokio.workspace = true
tokio-tungstenite.workspace = true
futures-util.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
clap.workspace = true
anyhow.workspace = true
serde_json.workspace = true
```

- [ ] **Step 2: Replace `spaze-server/src/lib.rs` with the real library shape**

```rust
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
use tracing::info;

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
    /// Construct shared state. Broadcast buffer of 64 — see DESIGN.md.
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
                        tracing::warn!("accept failed: {err}");
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
```

- [ ] **Step 3: Create the `connection` module stub**

Create `/Users/at-a/Repos/Spaze/spaze-server/src/connection.rs`:

```rust
//! Per-connection task. Real body lands in Task 5.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpStream;
use tracing::info;

use crate::{ConnectionId, ServerState};

/// Handle a single accepted TCP connection. Stub for now.
pub async fn handle_connection(
    _stream: TcpStream,
    _state: Arc<ServerState>,
    conn_id: ConnectionId,
    peer: SocketAddr,
) {
    info!("connection {} from {} accepted (stub — closing immediately)", conn_id.0, peer);
    // Real implementation in Task 5.
}
```

- [ ] **Step 4: Update `spaze-server/src/main.rs`**

```rust
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::Result;
use clap::Parser;
use spaze_server::{ServerConfig, run};

#[derive(Parser, Debug)]
#[command(version, about = "Spaze chat server daemon")]
struct Cli {
    /// Port to bind on (loopback only). Override binding interface in a future phase.
    #[arg(long, default_value_t = 9876)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), cli.port);

    run(ServerConfig { bind_addr }).await
}
```

- [ ] **Step 5: Run check + tests + lint**

Run: `cargo check --workspace --all-targets && cargo test -p spaze-server && cargo clippy -p spaze-server --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: 2 tests pass (`now_unix_ms_is_positive_and_advances`, `server_state_initializes_with_distinct_connection_ids`), all checks clean.

- [ ] **Step 6: Manual smoke-check (optional but useful)**

Run: `cargo run --bin spaze-server`
Expected: prints `listening on 127.0.0.1:9876`, then sits idle. Press Ctrl+C, expect `shutting down` log + clean exit (code 0).

If the binary crashes on startup, fix before continuing.

- [ ] **Step 7: Commit**

```bash
git add spaze-server/Cargo.toml spaze-server/src/lib.rs spaze-server/src/main.rs spaze-server/src/connection.rs
git commit -m "feat(server): scaffold ServerState, run loop, accept + ctrl_c" -m "Server binary now binds on 127.0.0.1:9876, accepts connections (currently stubbed — closes immediately), and exits cleanly on Ctrl+C. Connection task body lands in the next task."
```

---

## Task 5: `spaze-server` — `handle_connection` body + `handle_command` dispatch

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/spaze-server/src/connection.rs`

- [ ] **Step 1: Replace `connection.rs` with the full body**

```rust
//! Per-connection task: handshake, command dispatch, broadcast subscription.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use spaze_proto::{
    ClientCommand, ClientFrame, CommandOutcome, Message, MessageId, ProtocolError,
    ResponsePayload, ServerEvent, ServerFrame,
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
                        if let Err(err) = handle_text_frame(&text, &state, conn_id, &mut sink).await {
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
                        if let Err(err) = sink.send(WsMessage::Text(json.into())).await {
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
            body,
        } => {
            let msg = Message {
                id: MessageId::new(),
                room_id,
                author_id,
                author_device_id,
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
    sink.send(WsMessage::Text(json.into()))
        .await
        .context("ws sink write")?;
    Ok(())
}

// Type alias for the split sink half. Saves on long generic bounds elsewhere.
type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<TcpStream>,
    WsMessage,
>;
```

- [ ] **Step 2: Run check + clippy**

Run: `cargo check -p spaze-server && cargo clippy -p spaze-server --all-targets -- -D warnings`
Expected: clean. (No new tests yet; the integration test in Task 9 covers this.)

- [ ] **Step 3: Run fmt**

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 4: Manual smoke test (optional)**

In one terminal: `cargo run --bin spaze-server` — should print `listening on 127.0.0.1:9876`.
In another terminal (with `wscat` installed via `npm i -g wscat` if you have it, otherwise skip):
```bash
wscat -c ws://127.0.0.1:9876
> {"request_id":1,"command":{"type":"post_message","room_id":"00000000-0000-0000-0000-000000000000","author_id":"00000000-0000-0000-0000-000000000000","author_device_id":"00000000-0000-0000-0000-000000000000","body":{"kind":"text","content":"hi"}}}
< {"frame":"response","request_id":1,"result":{"outcome":"ok","payload":{"kind":"message_posted",...}}}
```

If `wscat` isn't available, this is fine — the integration test in Task 9 covers it automatically.

- [ ] **Step 5: Commit**

```bash
git add spaze-server/src/connection.rs
git commit -m "feat(server): implement handle_connection with broadcast dispatch" -m "Per-connection task does WS handshake, parses ClientFrame, dispatches PostMessage to broadcast (with Response back to sender per Option B), responds to other commands with InvalidRequest. Bad JSON returns Err response without dropping the connection."
```

---

## Task 6: `spaze-client` — identity helpers (TDD)

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/spaze-client/Cargo.toml`
- Create: `/Users/at-a/Repos/Spaze/spaze-client/src/lib.rs`
- Create: `/Users/at-a/Repos/Spaze/spaze-client/src/identity.rs`

- [ ] **Step 1: Update `spaze-client/Cargo.toml`**

Replace the existing `[dependencies]` block:

```toml
[dependencies]
spaze-proto = { path = "../spaze-proto" }
tokio.workspace = true
tokio-tungstenite.workspace = true
futures-util.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
clap.workspace = true
anyhow.workspace = true
serde_json.workspace = true
uuid.workspace = true
```

Full file becomes:

```toml
[package]
name = "spaze-client"
description = "Spaze TUI client"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true

[lints]
workspace = true

[[bin]]
name = "spaze"
path = "src/main.rs"

[dependencies]
spaze-proto = { path = "../spaze-proto" }
tokio.workspace = true
tokio-tungstenite.workspace = true
futures-util.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
clap.workspace = true
anyhow.workspace = true
serde_json.workspace = true
uuid.workspace = true
```

- [ ] **Step 2: Create `spaze-client/src/lib.rs` with the failing test**

```rust
//! `spaze-client` — TUI client for the Spaze chat protocol.
//!
//! Phase 1.A scope: stdin/stdout chat, single hardcoded Room.

pub mod identity;

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
        // 1.A simplification: DeviceId == UserId for the same name.
        // Phase 3 will introduce real device keypairs and untangle them.
        let (user, device) = derive_identity("andreas");
        assert_eq!(user.as_uuid(), device.as_uuid());
    }
}
```

- [ ] **Step 3: Run the failing tests**

Run: `cargo test -p spaze-client`
Expected: compile failure — `identity` module does not exist.

- [ ] **Step 4: Create `spaze-client/src/identity.rs`**

```rust
//! Client identity derivation.
//!
//! Phase 1.A: identity is purely deterministic from the `--name` flag.
//! `UserId` and `DeviceId` are derived via UUIDv5 in a hardcoded namespace.
//! Phase 3 introduces real GitHub-backed identity and per-device Ed25519 keys.

use spaze_proto::{DeviceId, UserId};
use uuid::{Uuid, uuid};

/// Hardcoded namespace UUID for Spaze identity derivation.
///
/// Generated once via `uuidgen` for this project. Stable across all releases —
/// changing it would re-shuffle every name's derived `UserId`.
const SPAZE_NAMESPACE: Uuid = uuid!("8e7a9f1c-3d52-4e8b-9c1d-7f6a8b9c0d1e");

/// Derive `(UserId, DeviceId)` from a display name.
///
/// In Phase 1.A, `DeviceId == UserId` for the same name — there's no real
/// device-vs-user distinction yet. Phase 3 splits them when device keypairs
/// land.
#[must_use]
pub fn derive_identity(name: &str) -> (UserId, DeviceId) {
    let user_uuid = Uuid::new_v5(&SPAZE_NAMESPACE, name.as_bytes());
    (UserId::from_uuid(user_uuid), DeviceId::from_uuid(user_uuid))
}
```

- [ ] **Step 5: Run tests — verify they pass**

Run: `cargo test -p spaze-client`
Expected: 4 tests pass.

- [ ] **Step 6: Run lint + fmt**

Run: `cargo clippy -p spaze-client --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: both clean.

- [ ] **Step 7: Commit**

```bash
git add spaze-client/Cargo.toml spaze-client/src/lib.rs spaze-client/src/identity.rs
git commit -m "feat(client): derive_identity via UUIDv5 from --name" -m "Stable per-name UserId and DeviceId. In 1.A, DeviceId equals UserId; Phase 3 splits them when real device keys arrive."
```

---

## Task 7: `spaze-client` — `render` module + `connect()` library entrypoint

**Files:**
- Create: `/Users/at-a/Repos/Spaze/spaze-client/src/render.rs`
- Modify: `/Users/at-a/Repos/Spaze/spaze-client/src/lib.rs`

- [ ] **Step 1: Create `spaze-client/src/render.rs`**

```rust
//! Frame rendering for the Phase 1.A stdin/stdout client.
//!
//! Phase 1.B replaces these with TUI widgets.

use spaze_proto::{
    CommandOutcome, Message, MessageBody, ResponsePayload, ServerEvent, ServerFrame,
};

/// Render a `ServerFrame` to stdout/stderr.
///
/// - `Response::Ok(MessagePosted)` and `Event(MessagePosted)` both render the
///   message line.
/// - `Response::Err` writes to stderr.
/// - Other variants are no-ops in 1.A.
pub fn render_frame(frame: &ServerFrame) {
    match frame {
        ServerFrame::Response { result, .. } => match result {
            CommandOutcome::Ok {
                payload: ResponsePayload::MessagePosted(msg),
            } => print_message(msg),
            CommandOutcome::Ok {
                payload: ResponsePayload::Empty,
            } => {}
            CommandOutcome::Err { error } => eprintln!("error: {error}"),
        },
        ServerFrame::Event(ServerEvent::MessagePosted(msg)) => print_message(msg),
        ServerFrame::Event(_) => {
            // Other events (UserJoined, Typing, etc.) deferred to Phase 2.
        }
    }
}

/// Print a single message in the 1.A "<short_uuid> body" format.
pub fn print_message(msg: &Message) {
    let short = format!("{:.8}", msg.author_id.as_uuid().simple());
    match &msg.body {
        MessageBody::Text { content } => println!("<{short}> {content}"),
        MessageBody::System { content } => println!("-- system: {content}"),
    }
}
```

- [ ] **Step 2: Add `connect()` library entrypoint to `spaze-client/src/lib.rs`**

Replace `spaze-client/src/lib.rs` with:

```rust
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
                                body: MessageBody::Text { content: line },
                            },
                        };
                        let json = serde_json::to_string(&frame)
                            .context("serialize ClientFrame")?;
                        sink.send(WsMessage::Text(json.into()))
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
```

- [ ] **Step 3: Run check + clippy**

Run: `cargo check -p spaze-client && cargo clippy -p spaze-client --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 4: Run tests**

Run: `cargo test -p spaze-client`
Expected: 4 identity tests pass (no new tests in this task — the connect() loop is exercised by the integration test in Task 9).

- [ ] **Step 5: Run fmt**

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add spaze-client/src/lib.rs spaze-client/src/render.rs
git commit -m "feat(client): connect() library entrypoint with render module" -m "Library exposes ClientConfig + run() that drives the stdin/WS select loop. Render module prints messages with 8-char author prefix and writes errors to stderr. main.rs becomes a thin CLI wrapper in the next task."
```

---

## Task 8: `spaze-client` — thin `main.rs` wrapper

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/spaze-client/src/main.rs`

- [ ] **Step 1: Replace `spaze-client/src/main.rs`**

```rust
use anyhow::Result;
use clap::Parser;
use spaze_client::{ClientConfig, identity::derive_identity, run};
use spaze_proto::RoomId;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(version, about = "Spaze TUI client (Phase 1.A: stdin/stdout)")]
struct Cli {
    /// Display name. Defaults to $USER, or "anon" if unset.
    #[arg(long)]
    name: Option<String>,

    /// WebSocket URL of the spaze-server.
    #[arg(long, default_value = "ws://127.0.0.1:9876/")]
    server: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let name = cli
        .name
        .unwrap_or_else(|| std::env::var("USER").unwrap_or_else(|_| "anon".to_string()));
    let (user_id, device_id) = derive_identity(&name);

    // Hardcoded room ID for Phase 1.A. Server uses its own; both ends agree to put
    // messages in this single Room. Server doesn't currently care which RoomId
    // a message is tagged with — it broadcasts everything.
    let room_id = RoomId::from_uuid(Uuid::nil());

    let config = ClientConfig {
        server_url: cli.server,
        user_id,
        device_id,
        room_id,
    };

    run(config).await
}
```

The hardcoded `RoomId::from_uuid(Uuid::nil())` is intentional and documented — Phase 2 replaces it with real Room management. Server doesn't validate `RoomId` in 1.A.

- [ ] **Step 2: Run check + clippy + fmt + tests**

Run: `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check && cargo test --workspace --all-targets`
Expected: 14 proto tests + 4 client identity tests + 2 server tests = 20 total, all pass.

- [ ] **Step 3: End-to-end manual smoke test**

Open three terminals:

Terminal 1: `cargo run --bin spaze-server`
- Expected: `listening on 127.0.0.1:9876`

Terminal 2: `cargo run --bin spaze -- --name andreas`
- Expected: connects, no errors

Terminal 3: `cargo run --bin spaze -- --name beth`
- Expected: connects, no errors

In Terminal 2, type `hello world` and press Enter.
- Expected in Terminal 2: `<a3...>` (8-char prefix) `hello world`
- Expected in Terminal 3: same line

In Terminal 3, type `hi back` and press Enter.
- Expected in Terminal 3: `<short> hi back`
- Expected in Terminal 2: same line

Press Ctrl+C in Terminal 2.
- Expected: client exits cleanly (code 0). Server keeps running.

Press Ctrl+C in Terminal 1 (server).
- Expected: server logs "shutting down", exits.
- In Terminal 3, expect: `connection lost: server closed`, exit code 1.

If any of these fail, fix before continuing. The integration test in Task 9 automates these checks but the manual run validates the human-facing UX.

- [ ] **Step 4: Commit**

```bash
git add spaze-client/src/main.rs
git commit -m "feat(client): thin main.rs CLI wrapper over spaze_client::run" -m "Resolves --name (or \$USER, or 'anon') into a stable UserId/DeviceId via UUIDv5, then hands off to the library's run(). Hardcoded RoomId::nil() for 1.A — Phase 2 replaces with real Room management."
```

---

## Task 9: Integration test — two clients chat through one server

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/spaze-server/Cargo.toml` (add dev-deps)
- Create: `/Users/at-a/Repos/Spaze/spaze-server/tests/two_client_chat.rs`

- [ ] **Step 1: Add dev-deps to `spaze-server/Cargo.toml`**

Append the following to `/Users/at-a/Repos/Spaze/spaze-server/Cargo.toml` after the existing `[dependencies]` block:

```toml
[dev-dependencies]
spaze-client = { path = "../spaze-client" }
spaze-proto = { path = "../spaze-proto" }
tokio = { workspace = true, features = ["full", "test-util"] }
serde_json.workspace = true
anyhow.workspace = true
futures-util.workspace = true
tokio-tungstenite.workspace = true
```

- [ ] **Step 2: Create `spaze-server/tests/two_client_chat.rs`**

The integration test drives two raw WebSocket connections (rather than full client `run` loops, which read stdin and would block tests). This lets us assert exact frame contents.

```rust
//! End-to-end integration test for Phase 1.A.
//!
//! Spawns the server in-process on an OS-assigned port, then opens two raw
//! WebSocket connections and asserts the message flow:
//!
//! 1. Client A sends PostMessage("hello") → A receives Response, B receives Event.
//! 2. Client B sends PostMessage("hi back") → B receives Response, A receives Event.

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
            body: MessageBody::Text {
                content: "hello".to_string(),
            },
        },
    };
    a_ws.send(WsMessage::Text(serde_json::to_string(&frame_a)?.into()))
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
        other => return Err(anyhow!("expected Response, got {other:?}")),
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
            body: MessageBody::Text {
                content: "hi back".to_string(),
            },
        },
    };
    b_ws.send(WsMessage::Text(serde_json::to_string(&frame_b)?.into()))
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
    let listener = tokio::net::TcpListener::bind(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        0,
    ))
    .await?;
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
                matches!(
                    error,
                    spaze_proto::ProtocolError::InvalidRequest { .. }
                ),
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
            body: MessageBody::Text {
                content: "still here".to_string(),
            },
        },
    };
    ws.send(WsMessage::Text(serde_json::to_string(&frame)?.into()))
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
            let frame: ServerFrame =
                serde_json::from_str(&text).context("parse ServerFrame")?;
            Ok(frame)
        }
        other => Err(anyhow!("unexpected WS message type: {other:?}")),
    }
}
```

Note on the `state.clone()` calls: `ServerState::new()` returns `Arc<ServerState>`, so `clone()` is the cheap pointer clone. This needs `ServerState` to be reachable from a public path — see Step 3 below.

- [ ] **Step 3: Make sure `ServerState` and `ConnectionId` are reachable from `spaze_server::*`**

Verify `spaze-server/src/lib.rs` already has `pub struct ServerState` and `pub struct ConnectionId`. (They were defined as `pub` in Task 4 — this is a confirmation step.)

If they aren't `pub`, make them `pub`. Same for the `connection` module — the test imports `spaze_server::connection::handle_connection`, which requires `pub mod connection;` and `pub async fn handle_connection`.

Also verify that `ServerState::new()` returns `Arc<Self>` — the test relies on `state.clone()` being cheap.

- [ ] **Step 4: Run the integration test**

Run: `cargo test -p spaze-server --test two_client_chat`
Expected: 2 tests pass:
  - `two_clients_can_chat`
  - `invalid_json_returns_invalid_request_without_dropping_connection`

If the test hangs, the most likely cause is the server task not seeing the close frames — `server_handle.abort()` should clean up. If a timeout fires, increase `STEP_TIMEOUT` only as a last resort; the symptom usually means a missing `select!` arm.

- [ ] **Step 5: Run the full workspace verification**

Run: `cargo test --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected:
- 14 proto tests + 4 client identity tests + 2 server unit tests + 2 integration tests = 22 tests, all green.
- Clippy and fmt clean.

- [ ] **Step 6: Commit**

```bash
git add spaze-server/Cargo.toml spaze-server/tests/two_client_chat.rs
git commit -m "test(server): integration test for two-client chat" -m "Drives two raw WS connections through the in-process server, asserts the PostMessage round-trip (Response to sender, Event to other) plus invalid-JSON resilience (InvalidRequest response without dropping the connection)."
```

---

## Task 10: README highlight pass + DESIGN.md status note

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/README.md`
- Modify: `/Users/at-a/Repos/Spaze/DESIGN.md`

This task keeps the docs honest. README currently says "pre-alpha. Bootstrapping the workspace. Not usable yet." — that becomes inaccurate the moment 1.A merges. DESIGN.md's Phased Scope table doesn't currently track sub-projects.

- [ ] **Step 1: Update `README.md` status line**

Find this line in `/Users/at-a/Repos/Spaze/README.md`:

```markdown
> **Status:** pre-alpha. Bootstrapping the workspace. Not usable yet.
```

Replace with:

```markdown
> **Status:** pre-alpha. Phase 1.A complete — bare WS chat works in two terminals. TUI, theming, slash commands, auth still ahead.
```

Also update the `## Building` section. Currently:

```markdown
Requires Rust 1.85 or newer.

\`\`\`bash
cargo build --workspace
\`\`\`
```

Append a `## Running` section after `## Building`:

```markdown
## Running (Phase 1.A bare-WS demo)

Three terminals — one server, two clients:

\`\`\`bash
# Terminal 1
cargo run --bin spaze-server

# Terminal 2
cargo run --bin spaze -- --name andreas

# Terminal 3
cargo run --bin spaze -- --name beth
\`\`\`

Type a line in either client; both terminals will see it. Author shows as the first 8 hex chars of the UUIDv5-derived UserId. Ctrl+C exits cleanly.
```

(Note: the surrounding markdown fences in this plan use real backticks — escape them when transcribing if your editor mangles them.)

- [ ] **Step 2: Update `DESIGN.md` Phased Scope row 1**

Find this row in `/Users/at-a/Repos/Spaze/DESIGN.md`:

```markdown
| 1 | 1 | Workspace, `spaze-proto` (IDs/errors/messages/events with correlation frames), server + client skeletons, **buffer abstraction** in TUI from day one, **slash command parser**, **theming module scaffold**, single hardcoded Space/Room, plaintext WebSocket — two terminals chat |
```

Replace with:

```markdown
| 1 | 1 | Workspace, `spaze-proto`, server + client skeletons, buffer abstraction, slash command parser, theming scaffold, single hardcoded Space/Room, plaintext WebSocket — two terminals chat. **Decomposed into 1.A (bare WS chat ✅), 1.B (TUI + buffer + theming), 1.C (slash commands).** |
```

Also append a new section to `DESIGN.md` immediately before the `## Roadmap (post-1.0)` section:

```markdown
## Sub-project tracking

Phase 1 was decomposed during brainstorming into three sub-projects, each with its own spec under `docs/superpowers/specs/` and plan under `docs/superpowers/plans/`:

| Sub-project | Status | Spec | Plan |
|---|---|---|---|
| 1.A — WS round-trip MVP | ✅ shipped | `2026-05-04-phase-1a-ws-mvp-design.md` | `2026-05-04-phase-1a-ws-mvp.md` |
| 1.B — TUI shell + buffer + theming scaffold | not yet planned | — | — |
| 1.C — Slash command parser | not yet planned | — | — |

```

- [ ] **Step 3: Verify CI still passes locally**

Run: `cargo test --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: 22 tests pass, all checks clean.

- [ ] **Step 4: Commit**

```bash
git add README.md DESIGN.md
git commit -m "docs: mark Phase 1.A complete in README + DESIGN" -m "README status updated, running instructions added. DESIGN.md tracks sub-project decomposition for Phase 1."
```

---

## Task 11: Push branch and verify CI on GitHub

**Files:** None.

- [ ] **Step 1: Verify all commits**

Run: `git log --oneline main..HEAD`
Expected: a sequence of commits matching the tasks above (around 9 commits — one per task, plus possibly a few smaller adjustments).

If something looks off (e.g., unintended commits, commit messages with `Co-Authored-By`), fix before pushing. To remove a stray `Co-Authored-By` line you'd `git rebase -i main` and reword the offending commit — but if the previous tasks were followed, this shouldn't happen.

- [ ] **Step 2: Push the branch**

```bash
git push -u origin phase-1a
```

Expected: push completes, prints branch tracking info.

- [ ] **Step 3: Watch the CI run for the branch**

```bash
sleep 5 && gh run list --limit 3
```

Then either watch live or wait:

```bash
gh run watch $(gh run list --limit 1 --json databaseId -q '.[0].databaseId') --exit-status
```

Expected: all four jobs (`check`, `test`, `clippy`, `fmt`) pass within ~30 seconds each.

If any job fails, it's almost always:
- A clippy warning that didn't surface locally (different toolchain default features)
- A formatter difference

Reproduce locally with `cargo +stable clippy --workspace --all-targets -- -D warnings` or `cargo +stable fmt --all -- --check`. Fix, recommit, push.

---

## Task 12: Open the PR

**Files:** None.

- [ ] **Step 1: Open the PR**

```bash
gh pr create --base main --head phase-1a \
  --title "Phase 1.A: bare WS chat — two terminals talk through one server" \
  --body "$(cat <<'EOF'
## Summary

First sub-project of Phase 1. Bare server + bare client over plaintext WebSocket, single hardcoded Room, stdin/stdout chat. No TUI yet (1.B), no slash commands (1.C), no auth (Phase 3), no persistence (Phase 2).

Three locked design decisions from brainstorming:
- **Identity:** \`spaze --name <NAME>\` → stable UserId/DeviceId via UUIDv5.
- **Correlation behavior:** server sends Response to sender, broadcasts Event to others (sender skipped via ConnectionId).
- **Server is identity-agnostic:** no presence events in 1.A; server only knows what clients put on each PostMessage.

Spec: \`docs/superpowers/specs/2026-05-04-phase-1a-ws-mvp-design.md\`
Plan: \`docs/superpowers/plans/2026-05-04-phase-1a-ws-mvp.md\`

## Test plan

- [x] \`cargo test --workspace --all-targets\` — 22 green
- [x] \`cargo clippy --workspace --all-targets -- -D warnings\` — clean
- [x] \`cargo fmt --all -- --check\` — clean
- [x] Manual: two-terminals demo (see README \`## Running\`)
- [x] Integration test: two clients chat through one server, plus invalid-JSON resilience
- [x] Ctrl+C on client exits 0; server keeps running
- [x] Ctrl+C on server exits 0; clients see "connection lost" and exit 1

## Wire protocol changes

- \`ClientCommand::PostMessage\` gains \`author_id\` and \`author_device_id\` fields. Workspace version bumped 0.1.0 → 0.2.0. Two existing proto tests updated; one new test added (\`post_message_carries_author_fields\`).

## Out of scope (next sub-projects)

- 1.B — TUI shell, buffer abstraction, theming scaffold
- 1.C — Slash command parser

EOF
)"
```

(Note: the heredoc `EOF` form preserves backticks. If your shell mangles them, fall back to inline `--body "..."` with escaping.)

Expected: prints the PR URL.

- [ ] **Step 2: Verify the PR**

Open the PR URL in a browser. Confirm:
- Title is meaningful.
- Body renders correctly (markdown, not raw text).
- CI is running or already passed (4 green checks: check, test, clippy, fmt).
- Diff is sensible.

- [ ] **Step 3: Self-merge or wait for review**

Per Andreas's CLAUDE.md, this is a self-merge situation (single author). Either:
- Self-merge via the GitHub web UI ("Squash and merge" or "Merge").
- Or use `gh pr merge --squash` (or `--merge`) to do it from the CLI.

After merge:

```bash
git checkout main
git pull origin main
git branch -d phase-1a
git push origin --delete phase-1a   # optional: also delete remote branch
```

---

## Done

After Task 12, Phase 1.A is shipped on `main`. Next: brainstorm and plan **1.B** — TUI shell, the foundational `Buffer` trait, and theming scaffold. That's a separate session with its own spec and plan.
