# Spaze Phase 1.A — WS Round-Trip MVP — Design

> **Status:** design locked, ready for implementation plan.
> **Sub-project of:** Phase 1 (week 1 of the 8-week MVP).
> **Sibling specs to come:** 1.B (TUI shell + buffer abstraction + theming scaffold), 1.C (slash command parser).

## Context

Phase 1 of the Spaze MVP — as written in `DESIGN.md`'s Phased Scope table — combines server skeleton, client TUI skeleton, buffer abstraction, slash commands, and theming scaffold into a single one-week deliverable. For a side-project pace that is too much for one implementation plan. Phase 1 has been decomposed into three plan-sized sub-projects, each producing working software that can be committed and walked away from:

- **1.A — WS round-trip MVP** (this spec). Bare server + bare client, no TUI, single hardcoded Room, plaintext WebSocket. Two terminals can chat through the server via stdin/stdout.
- **1.B — TUI shell with buffer abstraction + theming scaffold.** Replaces stdin/stdout with a `ratatui` TUI. Defines the foundational `Buffer` trait. One built-in theme (Catppuccin Mocha) hardcoded; full theme system arrives in Phase 4.
- **1.C — Slash command parser.** `spaze-commands` crate gets a registry + dispatcher. Starter commands (`/quit`, `/help`, `/connect`). Wired into the buffer's input dispatch.

Each sub-project gets its own spec → plan → implementation cycle.

## Goal of 1.A

Prove the wire format end-to-end. Two terminals chat through one server. Both sides type via stdin and read responses on stdout. No TUI, no theming, no slash commands, no auth, no persistence. The integration test that closes 1.A is **"start a server, connect two clients with different `--name`s, send a message from each, see it on both."**

## Scope

### In scope

- `spaze-server` binary that broadcasts within a single hardcoded Room.
- `spaze` (client) binary that reads stdin, sends `PostMessage` commands, prints incoming `MessagePosted` events.
- Plaintext WebSocket transport (no TLS).
- Client identity via `--name <NAME>` flag, deterministically hashed to a stable `UserId`/`DeviceId` (UUIDv5 in a hardcoded namespace).
- Request/response correlation per the proto already shipped: server sends `ServerFrame::Response` to the originating sender and broadcasts `ServerFrame::Event` to all *other* connected clients (sender skipped from broadcast via `ConnectionId` match).
- Integration test exercising real WS code paths in-process (not subprocess).

### Out of scope (deferred)

| Feature | Lands in |
|---|---|
| TUI rendering | Phase 1.B |
| Slash commands | Phase 1.C |
| Theming | Phase 1.B (scaffold) / Phase 4 (full) |
| GitHub OAuth + Ed25519 device keys | Phase 3 |
| SQLite persistence | Phase 2 |
| TLS | Phase 2 |
| Multiple Spaces / multiple Rooms | Phase 2 |
| Reconnection logic | Phase 2 |
| `UserJoined` / `UserLeft` presence events | Phase 2 |
| `EditMessage` / `DeleteMessage` commands | Phase 2 |
| Typing indicators | Phase 2 |
| Rate limiting | Phase 6+ |

## Locked Decisions

These were settled in the brainstorming session that produced this spec.

1. **Client identity (Q1, Option A).** `spaze --name <NAME>` defaults to `$USER` (or `"anon"` if unset). Stable `UserId`/`DeviceId` derived via UUIDv5 in a hardcoded namespace UUID baked into the client binary. `DeviceId == UserId` in 1.A; Phase 3 untangles them when real device keys land.

2. **Correlation behavior on `PostMessage` (Q2, Option B).** Server sends `ServerFrame::Response { request_id, result: CommandOutcome::Ok { payload: ResponsePayload::MessagePosted(msg) } }` to the originating sender. Server broadcasts `ServerFrame::Event(ServerEvent::MessagePosted(msg))` to all *other* clients. Sender is excluded from the broadcast — they already received the message via `Response`. This avoids duplicate rendering on the sender's side and keeps wire semantics clean.

3. **No presence events in 1.A (Q3, Option B).** Server is identity-agnostic — it knows only what clients put on each `PostMessage`. No connect-time handshake, no `UserJoined`/`UserLeft` broadcasts, no per-connection identity tracking beyond the opaque `ConnectionId`. Real presence events arrive in Phase 2 alongside `JoinRoom`/`LeaveRoom`. Phase 3 lights up the user-friendly form ("**andreas** joined") via real GitHub display names.

4. **Bind / port (Q4, Option A).** Server listens on `127.0.0.1:9876` by default; client connects to `ws://127.0.0.1:9876/` by default. Override via `--port <N>` on the server, `--server <url>` on the client. `127.0.0.1` (loopback) is the safe localhost-only default — switching to `0.0.0.0` for LAN demos is a one-line change later.

## Crate layout

| Crate | Change |
|---|---|
| `spaze-proto` | **Bump 0.1.0 → 0.2.0.** Add `author_id` and `author_device_id` to `ClientCommand::PostMessage`. Update existing tests. |
| `spaze-server` | Real binary. New deps. Adds the `ServerState`, `handle_connection` task, command dispatch. |
| `spaze-client` | Real binary. New deps. Refactor: connection logic moves into `spaze-client/src/lib.rs`; `src/main.rs` becomes a thin CLI wrapper. This is required for the integration test to exercise client code in-process. |
| `spaze-crypto` | Untouched (Phase 3). |
| `spaze-storage` | Untouched (Phase 2). |
| `spaze-commands` | Untouched (Phase 1.C). |

### New workspace dependencies

Add to `[workspace.dependencies]` in the root `Cargo.toml`:

```toml
tokio = { version = "1", features = ["rt-multi-thread", "macros", "io-std", "signal", "net", "sync"] }
tokio-tungstenite = "0.24"
futures-util = { version = "0.3", default-features = false, features = ["sink"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
clap = { version = "4", features = ["derive"] }
anyhow = "1"
```

Both `spaze-server` and `spaze-client` reference these via `*.workspace = true`.

## Required proto change

`ClientCommand::PostMessage` gains two fields:

```rust
PostMessage {
    room_id: RoomId,
    author_id: UserId,             // NEW (1.A)
    author_device_id: DeviceId,    // NEW (1.A)
    body: MessageBody,
},
```

Adding fields to a struct variant of an enum that is not `#[non_exhaustive]` at the variant level is technically SemVer-breaking, but the proto is at `0.1.0` with the wire still in flux. **Bump `0.1.0` → `0.2.0`** in `[workspace.package]`. Note: this is a workspace-wide version bump that propagates to every member crate via `version.workspace = true`. The non-proto crates (`spaze-crypto`, `spaze-storage`, `spaze-commands`, `spaze-server`, `spaze-client`) move to `0.2.0` too. This is a deliberate "version coherence" choice — early in the project, having all crates share a version is useful; we can split them out later if reasons emerge. Two existing tests in `spaze-proto/src/lib.rs` need updating to include the new fields:

- `client_command_roundtrips_json` — adds `author_id`/`author_device_id` to its `PostMessage { ... }` literal.
- `client_frame_carries_request_id_and_command` — same.

A new test verifying `PostMessage` with author fields round-trips JSON cleanly is also worth adding. Total proto tests: 13 → 14 (or stay at 13 if the existing test is updated to also assert the new fields).

In Phase 3 the server validates `author_id` against the authenticated session and rejects mismatches. In 1.A the server trusts the values from the client.

## Server architecture

### Shared state

```rust
struct ServerState {
    /// Single broadcast channel because there is only one Room in 1.A.
    /// Each event carries the sending ConnectionId so subscribers can skip self.
    broadcast: tokio::sync::broadcast::Sender<(ConnectionId, ServerEvent)>,

    /// The hardcoded Room ID for 1.A. Phase 2 replaces with a registry.
    room_id: RoomId,

    /// Allocator for opaque per-connection IDs. NOT an authenticated identity —
    /// just a counter used to skip the sender on broadcast.
    next_connection_id: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConnectionId(u64);
```

Broadcast channel buffer: **64 slots**. Default tokio buffer of 16 is fine for the 2-client demo, but 64 is cheap insurance against any slow-subscriber lag and matches what we'd want in Phase 2 anyway.

### Top-level lifecycle

1. `fn main` → `tokio::main` (multi-threaded runtime).
2. Parse CLI: `--port` (default `9876`).
3. Initialize `tracing_subscriber` with `EnvFilter` (default `info`, `RUST_LOG` env var override).
4. Build `ServerState`, wrap in `Arc`.
5. Bind `TcpListener::bind(("127.0.0.1", port))`. Log the listening address.
6. Main loop:
   - `tokio::select!` between `listener.accept()` and `tokio::signal::ctrl_c()`.
   - On accept: mint a fresh `ConnectionId`, spawn `handle_connection(stream, Arc<ServerState>, conn_id)`.
   - On Ctrl+C: log `"shutting down"`, drop the listener, fall out of the loop.
7. Exit 0. **Existing connection tasks abort abruptly when the runtime drops** — clients see TCP close on their side and report `"connection lost"`. This is acceptable for 1.A; graceful shutdown (signal connections, wait for clean WS close frames, then exit) lands when persistence in Phase 2 makes it matter.

### Per-connection task (`handle_connection`)

1. Accept the WebSocket handshake: `tokio_tungstenite::accept_async(stream).await?`. On error, log `warn`, return.
2. Split the WS into `(sink, stream)` via `futures_util::StreamExt::split`.
3. Subscribe to broadcasts: `let mut rx = state.broadcast.subscribe();`.
4. Main loop with `tokio::select!`:
   - `Some(msg) = ws_stream.next()` → parse the WS message as `ClientFrame`. On parse error: send a `Response { result: Err(InvalidRequest { message: "..." }) }`, continue. On valid frame: dispatch via `handle_command`. On WS close / WS error: break.
   - `result = rx.recv()` →
     - `Ok((sender_id, event))` → if `sender_id != self_conn_id`, serialize as `ServerFrame::Event(event)` and send to sink.
     - `Err(RecvError::Lagged(n))` → log `warn` (`"slow subscriber lagged {n} events"`), continue receiving newer events.
     - `Err(RecvError::Closed)` → break (broadcast sender dropped — server shutting down).
5. On exit: log `info` `"connection closed"`, task ends. The `Arc<ServerState>` reference drops; the broadcast subscriber slot ages out naturally.

### Command dispatch (1.A only handles `PostMessage`)

```rust
async fn handle_command(
    frame: ClientFrame,
    state: &ServerState,
    conn_id: ConnectionId,
    ws_sink: &mut WsSink,
) -> Result<()> {
    match frame.command {
        ClientCommand::PostMessage { room_id, author_id, author_device_id, body } => {
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

            // Response to sender (the canonical Message — server-assigned ID and timestamp).
            let response = ServerFrame::Response {
                request_id: frame.request_id,
                result: CommandOutcome::Ok {
                    payload: ResponsePayload::MessagePosted(msg.clone()),
                },
            };
            ws_sink.send(Message::Text(serde_json::to_string(&response)?)).await?;

            // Broadcast Event to others. Subscribers skip self via the ConnectionId tag.
            // SendError (no subscribers) is harmless — the sender's own response already went out.
            let _ = state.broadcast.send((conn_id, ServerEvent::MessagePosted(msg)));
        }

        // Edit/Delete/Join/Leave/Typing not in 1.A — respond with an explicit error.
        _ => {
            let response = ServerFrame::Response {
                request_id: frame.request_id,
                result: CommandOutcome::Err {
                    error: ProtocolError::InvalidRequest {
                        message: "command not implemented in Phase 1.A".to_string(),
                    },
                },
            };
            ws_sink.send(Message::Text(serde_json::to_string(&response)?)).await?;
        }
    }
    Ok(())
}
```

### Error handling

- **Recoverable per-connection errors** (bad JSON, unknown command, internal hiccup): respond with `Response { result: Err(...) }`, log `warn`, keep connection alive.
- **Connection-level errors** (WS protocol violation, sink write failure, peer reset): log `info`, drop the connection cleanly. No server-wide crash.
- **Top-level errors** (port bind failure, invalid CLI, panic): error out with `anyhow::Result` context, exit non-zero.

## Client architecture

### CLI args (`clap` derive)

```rust
#[derive(clap::Parser)]
struct Cli {
    /// Display name (also derives stable UserId/DeviceId).
    /// Defaults to $USER, or "anon" if unset.
    #[arg(long)]
    name: Option<String>,

    /// WebSocket URL of the spaze-server.
    #[arg(long, default_value = "ws://127.0.0.1:9876/")]
    server: String,
}
```

### Identity derivation

```rust
// Generate once with `uuidgen` during implementation; bake in as a const.
const SPAZE_NAMESPACE: Uuid = uuid!("...placeholder, generate during impl...");

fn derive_identity(name: &str) -> (UserId, DeviceId) {
    let user_uuid = Uuid::new_v5(&SPAZE_NAMESPACE, name.as_bytes());
    // 1.A simplification: DeviceId == UserId.
    // Phase 3 derives DeviceId from a real device keypair.
    (UserId::from_uuid(user_uuid), DeviceId::from_uuid(user_uuid))
}
```

The namespace UUID is a constant the implementation generates once and commits to `spaze-client/src/identity.rs`. `--name andreas` always produces the same `UserId` across runs and across machines (that's the whole point of UUIDv5).

### Top-level lifecycle

1. `fn main` → `tokio::main`.
2. Parse CLI.
3. Resolve name: `--name` > `$USER` > `"anon"`.
4. Derive `(UserId, DeviceId)` from resolved name.
5. Initialize `tracing_subscriber`.
6. `connect_async(&cli.server).await?`. On error: print `"failed to connect: {err}"`, exit non-zero.
7. Split WS into `(sink, stream)`.
8. Wrap stdin: `let mut stdin = BufReader::new(tokio::io::stdin()).lines();`.
9. Initialize `request_id_counter: AtomicU64 = 1`.
10. Main loop with `tokio::select!`:
    - `Ok(Some(line)) = stdin.next_line()` → build `ClientFrame { request_id: counter.fetch_add(1, Relaxed), command: PostMessage { ... } }`, JSON-encode, send.
    - `Ok(None) = stdin.next_line()` → stdin EOF (e.g., piped input ended). Log `"stdin closed"` at info, send WS close frame, break, exit 0.
    - `Some(msg) = ws_stream.next()` → parse as `ServerFrame`, dispatch to renderer.
    - `_ = tokio::signal::ctrl_c()` → log shutdown, send WS close frame, break, exit 0.
11. WS error / server-initiated close (mid-loop) → print `"connection lost: {reason}"`, exit 1.

### Receive-side rendering

```rust
fn render(frame: ServerFrame) {
    match frame {
        ServerFrame::Response { result, .. } => match result {
            CommandOutcome::Ok { payload: ResponsePayload::MessagePosted(msg) } => {
                print_message(&msg); // sender renders own message via Response
            }
            CommandOutcome::Ok { payload: ResponsePayload::Empty } => {} // no-op
            CommandOutcome::Err { error } => eprintln!("error: {error}"),
        },
        ServerFrame::Event(ServerEvent::MessagePosted(msg)) => {
            print_message(&msg); // peers render via Event
        }
        ServerFrame::Event(_) => {} // 1.A doesn't emit other events
    }
}

fn print_message(msg: &Message) {
    let short = format!("{:.8}", msg.author_id.as_uuid().simple());
    match &msg.body {
        MessageBody::Text { content } => println!("<{short}> {content}"),
        MessageBody::System { content } => println!("-- system: {content}"),
    }
}
```

The 8-character hex prefix (git-short-hash style) is the 1.A author display. With ~4 billion distinct prefixes, collision risk is negligible for the demo. Phase 3 maps the underlying `UserId` to a GitHub display name.

## Connection lifecycle table

| Event | Server | Client |
|---|---|---|
| Connect | TCP accept → WS upgrade → spawn task → subscribe | `connect_async` → split → enter loop |
| Send `PostMessage` | Build canonical `Message` → Response to sender + broadcast Event to others | Send `ClientFrame`; render own msg via Response |
| Receive Event (peer's message) | N/A | Render via `print_message` |
| Server-side Ctrl+C | Drop listener, existing tasks finish naturally | (peer) WS close → "connection lost" → exit 1 |
| Client-side Ctrl+C | (peer) WS close, drops broadcast slot | Send WS close frame, exit 0 |
| Bad JSON from client | Respond `Err(InvalidRequest)`, keep connection | (skipped — client always sends valid JSON) |
| WS protocol violation | Drop connection, log `info` | Print error, exit 1 |
| Slow subscriber (`Lagged(n)`) | Log `warn`, continue receiving | (not applicable on client side) |

## Testing

### Unit tests

- `spaze-proto`: existing 13 tests + updates to `client_command_roundtrips_json` and `client_frame_carries_request_id_and_command` for the new `PostMessage` fields. Optionally: a new test specifically asserting that `PostMessage` with `author_id`/`author_device_id` round-trips JSON cleanly.
- `spaze-client`: small unit test for `derive_identity` — same name → same UUIDs; different names → different UUIDs; empty string handles cleanly.
- `spaze-server`: minimal unit tests where defensible (`now_unix_ms` etc.). Most server logic is exercised via the integration test.

### Integration test

Lives at `spaze-server/tests/two_client_chat.rs`. Exercises real WS code paths in-process (no subprocess).

1. Spawn `spaze-server` listening on port `0` (OS-assigned). Capture the actual port from the listener.
2. Open two in-process `spaze-client` connections via the library entrypoint at `spaze_client::connect`.
   - Client A: `--name andreas`.
   - Client B: `--name beth`.
3. Client A sends `PostMessage(Text { content: "hello" })`.
4. **Assert:** Client A receives a `Response` with `Ok { MessagePosted(msg) }` whose body matches `"hello"` and whose `author_id` matches A's derived UserId.
5. **Assert:** Client B receives an `Event(MessagePosted(msg))` with the same id, content, and author.
6. Client B sends `PostMessage(Text { content: "hi back" })`.
7. **Assert:** Client A receives an `Event` with the message; Client B receives a `Response` with the same message.
8. Both clients send WS close.
9. **Assert:** Server task completes cleanly.

This requires extracting the client's connection logic into `spaze-client/src/lib.rs` so the test can call it directly. The `main.rs` of `spaze-client` becomes a thin CLI wrapper around the library function. Same pattern as most well-structured Rust binaries.

## Acceptance criteria

1. `cargo test --workspace --all-targets` passes (existing 13 tests + updates + new client identity test + integration test).
2. `cargo clippy --workspace --all-targets -- -D warnings` clean.
3. `cargo fmt --all -- --check` clean.
4. The two-terminals demo works manually:
   - Terminal A: `cargo run --bin spaze-server`
   - Terminal B: `cargo run --bin spaze -- --name andreas`
   - Terminal C: `cargo run --bin spaze -- --name beth`
   - Type `hello` in B → see in B and C with `<a3c1f8b6>` style author prefix.
   - Type `hi` in C → see in B and C.
5. Ctrl+C on a client cleanly exits with code 0; the server keeps running and accepts new clients.
6. Ctrl+C on the server cleanly exits with code 0; existing client connections see `"connection lost"` and exit 1.
7. Sending invalid JSON on the WS (manual test via `wscat` or similar) returns a `Response { result: Err(InvalidRequest) }` and does not drop the connection.
8. CI on GitHub passes (`check`, `test`, `clippy`, `fmt` all green).

## Implementation order (for the plan)

The implementation plan should land work in this order. Each step is committable independently if useful.

1. **Proto bump** — smallest, foundation for everything else. Update `Cargo.toml` workspace version to `0.2.0`, add `author_id`/`author_device_id` to `PostMessage`, update tests.
2. **Workspace dependencies** — add `tokio`, `tokio-tungstenite`, `futures-util`, `tracing`, `tracing-subscriber`, `clap`, `anyhow` to `[workspace.dependencies]`.
3. **`spaze-server` skeleton** — CLI parsing, listener, accept loop. Verify it accepts WS connections without crashing (test with `wscat` or a tiny ad-hoc client).
4. **`spaze-server` command dispatch** — `handle_connection`, `handle_command`, broadcast logic.
5. **`spaze-client` library entrypoint** — `spaze-client/src/lib.rs` with the connection + select loop + renderer.
6. **`spaze-client` binary** — thin CLI wrapper over the library.
7. **End-to-end manual test** — two terminals, real chat. Iterate any rough edges.
8. **Integration test** — `tests/two_client_chat.rs`. Automate the demo.
9. **Final verification** — `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check`, push, watch CI go green.

## Open implementation questions

These don't affect the design but will need micro-decisions during implementation:

- **Namespace UUID value.** Generate during implementation (`uuidgen`) and document the chosen value in `spaze-client/src/identity.rs`.
- **Time source.** `std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64` is the obvious choice. Could wrap in a small `now_unix_ms()` helper in a shared location.
- **WS subprotocol header.** `tokio-tungstenite` allows specifying `Sec-WebSocket-Protocol`. Not needed for 1.A but worth picking a value (e.g., `spaze.v1`) early so Phase 2 doesn't have a wire-protocol ambiguity. **Decision deferred to implementation** — the simplest 1.A implementation skips this.
- **Tracing log levels.** Most server events log at `info`; recoverable errors at `warn`; fatal/loop-exit conditions at `error`. Standard choices.

These are noted so the implementer doesn't get stuck on them, not because they need design input.
