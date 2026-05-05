# Spaze Phase 1.B — TUI Shell + Buffer Abstraction + Theming Scaffold — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the stdin/stdout client of Phase 1.A with a real `ratatui` + `crossterm` TUI. Introduce the foundational `Buffer` enum (Room + Help variants in 1.B; rest deferred). Lay in the 7-region layout (topbar / sidebar / tab strip / room header / buffer / input / status bar). Theme module with one built-in (Catppuccin Mocha). Two terminals chat with author *names* (not hex prefixes) in a real TUI.

**Architecture:** All Phase 1.B work is in `spaze-client` plus a small wire-protocol additive change in `spaze-proto` and a one-line passthrough in `spaze-server`. The client gains: `app.rs` (state machine), `tui.rs` (layout + region rendering), `theme/` (Theme struct + Catppuccin Mocha), `buffers/` (enum + 2 variants). Main loop is `tokio::select!` over (crossterm events, WS frames, Ctrl+C) — same shape as 1.A but with crossterm replacing stdin and a render call after each event.

**Tech Stack:** Rust 2024 edition, ratatui 0.29, crossterm 0.28, tokio, tokio-tungstenite, futures-util, serde + serde_json (already in proto), tracing + tracing-subscriber, clap (derive), anyhow, uuid (already in workspace).

**Spec reference:** `docs/superpowers/specs/2026-05-05-phase-1b-tui-shell-design.md` (canonical).

**Branching:** All tasks land on a new feature branch `phase-1b`. PR to `main` at the end (Task 16). Per Andreas's CLAUDE.md, all commits should be presented for review; subagent-driven execution may proceed task-by-task with the user's blanket authorization given before execution starts (same pattern as Phase 1.A).

**Conventions:**
- Conventional Commits (`feat:`, `chore:`, `test:`, `refactor:`, `docs:`).
- First line ≤72 chars. Body optional.
- **No `Co-Authored-By` lines, ever** (per Andreas's CLAUDE.md).

---

## File Structure

This plan creates these files (all in `spaze-client/src/` unless noted):

```
Spaze/
├── Cargo.toml                                 # MODIFY — version 0.2.0 → 0.3.0; add ratatui, crossterm
├── spaze-proto/src/
│   ├── events.rs                              # MODIFY — PostMessage gains author_display_name
│   ├── messages.rs                            # MODIFY — Message gains author_display_name
│   └── lib.rs                                 # MODIFY — update existing tests + add new test
├── spaze-server/src/connection.rs             # MODIFY — pass author_display_name through
├── spaze-client/
│   ├── Cargo.toml                             # MODIFY — add ratatui, crossterm
│   └── src/
│       ├── lib.rs                             # MODIFY — run() loop uses crossterm event stream
│       ├── render.rs                          # DELETE — content moves into buffers/room_timeline.rs
│       ├── app.rs                             # CREATE — App state machine
│       ├── tui.rs                             # CREATE — layout primitives + region renderers
│       ├── theme/
│       │   ├── mod.rs                         # CREATE — Theme struct (19 slots)
│       │   └── catppuccin_mocha.rs            # CREATE — the one built-in
│       └── buffers/
│           ├── mod.rs                         # CREATE — Buffer enum + BufferKind + dispatch
│           ├── room_timeline.rs               # CREATE — RoomTimelineBuffer
│           └── help.rs                        # CREATE — HelpBuffer
├── DESIGN.md                                  # MODIFY — collapse 3-level hierarchy to 2
├── README.md                                  # MODIFY — Phase 1.B status
```

Responsibility split (per the spec):
- **`app.rs`** owns top-level state: open buffers, active index, sidebar visibility, mode, input buffer, theme, connection, identity.
- **`tui.rs`** owns rendering: `setup()`, `teardown()`, `draw()` and per-region helpers (`draw_topbar`, `draw_sidebar`, `draw_tab_strip`, `draw_room_header`, `draw_input`, `draw_statusbar`). Stateless given an `App` and a `Frame`.
- **`theme/mod.rs`** defines `Theme` struct. One file per built-in.
- **`buffers/mod.rs`** defines `Buffer` enum + `BufferKind`. One file per variant.

---

## Task 1: Branch, workspace deps, version bump

**Files:**
- Create branch: `phase-1b` (off `main`)
- Modify: `/Users/at-a/Repos/Spaze/Cargo.toml`

- [ ] **Step 1: Create the feature branch**

```bash
cd /Users/at-a/Repos/Spaze
git checkout main && git pull origin main
git checkout -b phase-1b
```

Expected: on `phase-1b`. Verify: `git branch --show-current` → `phase-1b`.

- [ ] **Step 2: Update root `Cargo.toml`**

Bump version 0.2.0 → 0.3.0 and add `ratatui` + `crossterm` to `[workspace.dependencies]`:

```toml
[workspace.package]
version = "0.3.0"
```

Append to `[workspace.dependencies]`:

```toml
ratatui = "0.29"
crossterm = "0.28"
```

The full updated `[workspace.dependencies]` block becomes:

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
ratatui = "0.29"
crossterm = "0.28"
```

- [ ] **Step 3: Verify the workspace still builds**

```bash
cargo check --workspace
```

Expected: clean compile. New deps resolved into `Cargo.lock`. No code uses them yet.

- [ ] **Step 4: Verify lints + fmt**

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Expected: both clean.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: bump to 0.3.0 and add ratatui/crossterm deps for phase 1.B"
```

---

## Task 2: Proto change — `author_display_name` on `PostMessage` and `Message` (TDD)

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/spaze-proto/src/events.rs`
- Modify: `/Users/at-a/Repos/Spaze/spaze-proto/src/messages.rs`
- Modify: `/Users/at-a/Repos/Spaze/spaze-proto/src/lib.rs` (update tests + add new test)

- [ ] **Step 1: Write the new failing test**

Add to the `tests` module in `/Users/at-a/Repos/Spaze/spaze-proto/src/lib.rs` after `post_message_carries_author_fields`:

```rust
#[test]
fn post_message_carries_display_name() {
    let cmd = ClientCommand::PostMessage {
        room_id: RoomId::new(),
        author_id: UserId::new(),
        author_device_id: DeviceId::new(),
        author_display_name: "andreas".to_string(),
        body: MessageBody::Text { content: "hi".into() },
    };
    let json = serde_json::to_string(&cmd).unwrap();
    assert!(
        json.contains("\"author_display_name\":\"andreas\""),
        "display name missing in {json}"
    );
    let parsed: ClientCommand = serde_json::from_str(&json).unwrap();
    match parsed {
        ClientCommand::PostMessage { author_display_name, .. } => {
            assert_eq!(author_display_name, "andreas");
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn message_carries_display_name() {
    let msg = Message {
        id: MessageId::new(),
        room_id: RoomId::new(),
        author_id: UserId::new(),
        author_device_id: DeviceId::new(),
        author_display_name: "beth".to_string(),
        created_at_ms: 1_700_000_000_000,
        edited_at_ms: None,
        deleted_at_ms: None,
        body: MessageBody::Text { content: "hello".into() },
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.author_display_name, "beth");
}
```

- [ ] **Step 2: Run the new tests — verify they fail**

```bash
cargo test -p spaze-proto post_message_carries_display_name message_carries_display_name
```

Expected: compile failure — `author_display_name` field doesn't exist on `PostMessage` or `Message`.

- [ ] **Step 3: Modify `Message` in `messages.rs`**

Find the `Message` struct in `/Users/at-a/Repos/Spaze/spaze-proto/src/messages.rs`. Add `author_display_name: String` between `author_device_id` and `created_at_ms`:

```rust
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub room_id: RoomId,
    pub author_id: UserId,
    pub author_device_id: DeviceId,
    pub author_display_name: String,
    pub created_at_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_at_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at_ms: Option<i64>,
    pub body: MessageBody,
}
```

- [ ] **Step 4: Modify `PostMessage` in `events.rs`**

Find `ClientCommand::PostMessage`. Add `author_display_name: String`:

```rust
PostMessage {
    room_id: RoomId,
    author_id: UserId,
    author_device_id: DeviceId,
    author_display_name: String,
    body: MessageBody,
},
```

- [ ] **Step 5: Update existing tests in `lib.rs`**

Two existing tests construct `PostMessage` literals and need the new field:

`client_command_roundtrips_json`:

```rust
let cmd = ClientCommand::PostMessage {
    room_id: RoomId::new(),
    author_id: UserId::new(),
    author_device_id: DeviceId::new(),
    author_display_name: "alice".to_string(),
    body: MessageBody::Text { content: "hi".into() },
};
```

`client_frame_carries_request_id_and_command`:

```rust
let frame = ClientFrame {
    request_id: RequestId(42),
    command: ClientCommand::PostMessage {
        room_id: RoomId::new(),
        author_id: UserId::new(),
        author_device_id: DeviceId::new(),
        author_display_name: "alice".to_string(),
        body: MessageBody::Text { content: "hi".into() },
    },
};
```

`post_message_carries_author_fields`:

```rust
let user = UserId::new();
let device = DeviceId::new();
let cmd = ClientCommand::PostMessage {
    room_id: RoomId::new(),
    author_id: user,
    author_device_id: device,
    author_display_name: "alice".to_string(),
    body: MessageBody::Text { content: "hello".into() },
};
```

Also `server_event_message_posted_roundtrips` constructs a `Message`:

```rust
let msg = Message {
    id: MessageId::new(),
    room_id: RoomId::new(),
    author_id: UserId::new(),
    author_device_id: DeviceId::new(),
    author_display_name: "alice".to_string(),
    created_at_ms: 1_700_000_000_000,
    edited_at_ms: None,
    deleted_at_ms: None,
    body: MessageBody::Text { content: "hello".into() },
};
```

And `message_roundtrips_json`:

```rust
let msg = Message {
    id: MessageId::new(),
    room_id: RoomId::new(),
    author_id: UserId::new(),
    author_device_id: DeviceId::new(),
    author_display_name: "alice".to_string(),
    created_at_ms: 1_700_000_000_000,
    edited_at_ms: None,
    deleted_at_ms: None,
    body: MessageBody::Text { content: "hello world".into() },
};
```

And `edited_message_carries_edit_timestamp`:

```rust
let msg = Message {
    id: MessageId::new(),
    room_id: RoomId::new(),
    author_id: UserId::new(),
    author_device_id: DeviceId::new(),
    author_display_name: "alice".to_string(),
    created_at_ms: 1_700_000_000_000,
    edited_at_ms: Some(1_700_000_500_000),
    deleted_at_ms: None,
    body: MessageBody::Text { content: "edited content".into() },
};
```

- [ ] **Step 6: Run all proto tests**

```bash
cargo test -p spaze-proto
```

Expected: 16 tests pass total (the previous 14 plus the 2 new ones — `post_message_carries_display_name` and `message_carries_display_name`).

- [ ] **Step 7: Run clippy + fmt**

```bash
cargo clippy -p spaze-proto --all-targets -- -D warnings
cargo fmt --all -- --check
```

Expected: both clean.

- [ ] **Step 8: Commit**

```bash
git add spaze-proto/src/events.rs spaze-proto/src/messages.rs spaze-proto/src/lib.rs
git commit -m "feat(proto): add author_display_name to PostMessage and Message" -m "Required by Phase 1.B — TUI renders author names instead of 8-char hex prefixes. Trusted-from-client until Phase 3 auth validates against GitHub session."
```

---

## Task 3: Server passes `author_display_name` through

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/spaze-server/src/connection.rs`
- Modify: `/Users/at-a/Repos/Spaze/spaze-server/tests/two_client_chat.rs`

- [ ] **Step 1: Update `handle_command` in `connection.rs`**

Find the `PostMessage` arm in `handle_command`. The destructure currently is:

```rust
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
    // …
}
```

Update to pull and forward `author_display_name`:

```rust
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
    // … rest unchanged (Response + broadcast)
}
```

- [ ] **Step 2: Update the integration test fixtures**

`/Users/at-a/Repos/Spaze/spaze-server/tests/two_client_chat.rs` constructs three `PostMessage` literals. Each needs `author_display_name` added.

In `two_clients_can_chat`:

```rust
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
```

```rust
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
```

In `invalid_json_returns_invalid_request_without_dropping_connection`:

```rust
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
```

- [ ] **Step 3: Run server + integration tests**

```bash
cargo test -p spaze-server
```

Expected: 4 server-side tests pass (2 unit + 2 integration). The integration tests should still pass — the wire shape is just slightly bigger.

- [ ] **Step 4: Run workspace verification**

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test --workspace --all-targets
```

Expected: 20 total tests pass (16 proto + 2 server unit + 2 server integration).

- [ ] **Step 5: Commit**

```bash
git add spaze-server/src/connection.rs spaze-server/tests/two_client_chat.rs
git commit -m "feat(server): pass author_display_name through PostMessage handler" -m "Server is identity-agnostic in 1.B — trusts the display name from the client. Phase 3 auth will validate against the GitHub session."
```

---

## Task 4: Theme module + Catppuccin Mocha (TDD)

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/spaze-client/Cargo.toml` (add ratatui, crossterm)
- Create: `/Users/at-a/Repos/Spaze/spaze-client/src/theme/mod.rs`
- Create: `/Users/at-a/Repos/Spaze/spaze-client/src/theme/catppuccin_mocha.rs`
- Modify: `/Users/at-a/Repos/Spaze/spaze-client/src/lib.rs` (add `pub mod theme;` + new test)

- [ ] **Step 1: Update `spaze-client/Cargo.toml` to add ratatui + crossterm**

Append to the existing `[dependencies]` block:

```toml
ratatui.workspace = true
crossterm.workspace = true
```

Full block:

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
ratatui.workspace = true
crossterm.workspace = true
```

- [ ] **Step 2: Write the failing test in `lib.rs`**

Add to `/Users/at-a/Repos/Spaze/spaze-client/src/lib.rs` `tests` module:

```rust
#[test]
fn theme_catppuccin_mocha_has_all_slots_set() {
    use super::theme::catppuccin_mocha::CATPPUCCIN_MOCHA;
    use super::theme::Theme;
    use ratatui::style::Color;

    // Pick a sentinel default. If any slot equals this, the slot was forgotten.
    let default = Color::Reset;

    let t: Theme = CATPPUCCIN_MOCHA;
    assert_ne!(t.background, default, "background slot not set");
    assert_ne!(t.surface, default, "surface slot not set");
    assert_ne!(t.overlay, default, "overlay slot not set");
    assert_ne!(t.foreground, default, "foreground slot not set");
    assert_ne!(t.muted, default, "muted slot not set");
    assert_ne!(t.subtle, default, "subtle slot not set");
    assert_ne!(t.primary, default, "primary slot not set");
    assert_ne!(t.success, default, "success slot not set");
    assert_ne!(t.warning, default, "warning slot not set");
    assert_ne!(t.error, default, "error slot not set");
    assert_ne!(t.info, default, "info slot not set");
    assert_ne!(t.mention, default, "mention slot not set");
    assert_ne!(t.link, default, "link slot not set");
    assert_ne!(t.border, default, "border slot not set");
    assert_ne!(t.border_focused, default, "border_focused slot not set");
    assert_ne!(t.tab_room, default, "tab_room slot not set");
    assert_ne!(t.tab_dm, default, "tab_dm slot not set");
    assert_ne!(t.tab_special, default, "tab_special slot not set");
    assert_ne!(t.tab_unread, default, "tab_unread slot not set");
    assert!(!t.syntect_theme_name.is_empty(), "syntect_theme_name not set");
}
```

Also at the top of `lib.rs`, near the existing `pub mod identity; pub mod render;` line, add:

```rust
pub mod theme;
```

- [ ] **Step 3: Run the failing test**

```bash
cargo test -p spaze-client theme_catppuccin_mocha_has_all_slots_set
```

Expected: compile failure — `theme` module doesn't exist.

- [ ] **Step 4: Create `spaze-client/src/theme/mod.rs`**

```rust
//! Spaze client theming — semantic color slots.
//!
//! Components reference `theme.foreground` not `Color::Cyan`. Each built-in
//! binds the slots to specific colors. Phase 1.B ships one built-in
//! (Catppuccin Mocha); Phase 4 ships seven more plus user-loadable themes.

use ratatui::style::Color;

pub mod catppuccin_mocha;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    // surfaces
    pub background: Color,
    pub surface: Color,
    pub overlay: Color,

    // text
    pub foreground: Color,
    pub muted: Color,
    pub subtle: Color,

    // state
    pub primary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // chat-specific
    pub mention: Color,
    pub link: Color,
    pub border: Color,
    pub border_focused: Color,

    // tab kind tints
    pub tab_room: Color,
    pub tab_dm: Color,
    pub tab_special: Color,
    pub tab_unread: Color,

    // for syntect (Phase 4)
    pub syntect_theme_name: &'static str,
}
```

- [ ] **Step 5: Create `spaze-client/src/theme/catppuccin_mocha.rs`**

Hex values from Catppuccin's official Mocha palette.

```rust
//! Catppuccin Mocha — Spaze's default dark theme.
//!
//! Colors: <https://catppuccin.com/palette#mocha>

use ratatui::style::Color;

use super::Theme;

pub const CATPPUCCIN_MOCHA: Theme = Theme {
    // surfaces
    background: Color::Rgb(0x1e, 0x1e, 0x2e), // base
    surface:    Color::Rgb(0x18, 0x18, 0x25), // mantle
    overlay:    Color::Rgb(0x31, 0x32, 0x44), // surface0

    // text
    foreground: Color::Rgb(0xcd, 0xd6, 0xf4), // text
    muted:      Color::Rgb(0x6c, 0x70, 0x86), // overlay0
    subtle:     Color::Rgb(0x45, 0x47, 0x5a), // surface1

    // state
    primary: Color::Rgb(0xcb, 0xa6, 0xf7), // mauve
    success: Color::Rgb(0xa6, 0xe3, 0xa1), // green
    warning: Color::Rgb(0xf9, 0xe2, 0xaf), // yellow
    error:   Color::Rgb(0xf3, 0x8b, 0xa8), // pink
    info:    Color::Rgb(0x89, 0xb4, 0xfa), // blue

    // chat-specific
    mention:        Color::Rgb(0xf5, 0xc2, 0xe7), // pink (lighter)
    link:           Color::Rgb(0x89, 0xdc, 0xeb), // sky
    border:         Color::Rgb(0x45, 0x47, 0x5a), // surface1
    border_focused: Color::Rgb(0xcb, 0xa6, 0xf7), // mauve

    // tab kind tints
    tab_room:    Color::Rgb(0x94, 0xe2, 0xd5), // teal
    tab_dm:      Color::Rgb(0xf9, 0xe2, 0xaf), // yellow
    tab_special: Color::Rgb(0xfa, 0xb3, 0x87), // peach
    tab_unread:  Color::Rgb(0xf3, 0x8b, 0xa8), // pink

    syntect_theme_name: "Catppuccin Mocha",
};
```

- [ ] **Step 6: Run the test**

```bash
cargo test -p spaze-client theme_catppuccin_mocha_has_all_slots_set
```

Expected: 1 test passes.

- [ ] **Step 7: Run lint + fmt + workspace test**

```bash
cargo clippy -p spaze-client --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test --workspace --all-targets
```

Expected: clean. Workspace count: 21 tests (16 proto + 4 server + 1 client identity → wait, the existing client tests should be 4 + the new theme test = 5). Actually current count: 16 proto + 2 server unit + 2 server integration + 4 existing client identity tests + 1 new theme test = 25 tests. Let me recount: proto has 14 tests already, +2 for display name = 16. Server has 2 unit + 2 integration = 4. Client has 4 identity tests + 1 new theme = 5. Total 25. So expected 25.

- [ ] **Step 8: Commit**

```bash
git add spaze-client/Cargo.toml spaze-client/src/lib.rs spaze-client/src/theme/
git commit -m "feat(client): theme module with Catppuccin Mocha as the one built-in" -m "19 semantic color slots (background, foreground, muted, primary, mention, etc.). Phase 4 will add 7 more built-ins and user-loadable themes from \$XDG_CONFIG_HOME/spaze/themes/."
```

---

## Task 5: Buffer enum + `HelpBuffer` + `RoomTimelineBuffer` skeleton

**Files:**
- Create: `/Users/at-a/Repos/Spaze/spaze-client/src/buffers/mod.rs`
- Create: `/Users/at-a/Repos/Spaze/spaze-client/src/buffers/help.rs`
- Create: `/Users/at-a/Repos/Spaze/spaze-client/src/buffers/room_timeline.rs`
- Modify: `/Users/at-a/Repos/Spaze/spaze-client/src/lib.rs` (`pub mod buffers;`)

- [ ] **Step 1: Create `buffers/mod.rs` with the enum and dispatch**

```rust
//! Buffer abstraction — every "view" in Spaze is a Buffer.
//!
//! Phase 1.B ships two variants (Room, Help). Phase 5+ adds Pins, Files,
//! Notez, Todos, Users, Settings as more variants.

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::theme::Theme;

pub mod help;
pub mod room_timeline;

pub use help::HelpBuffer;
pub use room_timeline::RoomTimelineBuffer;

pub enum Buffer {
    Room(RoomTimelineBuffer), // also used for DMs (kind=Direct on inner struct)
    Help(HelpBuffer),
    // Phase 5+: Pins(PinsBuffer), Files(FilesBuffer), Notez(NotezBuffer),
    //          Todos(TodosBuffer), Users(UsersBuffer), Settings(SettingsBuffer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferKind {
    Room,
    DirectMessage,
    Pins,
    Files,
    Notez,
    Todos,
    Users,
    Help,
    Settings,
}

impl Buffer {
    pub fn title(&self) -> &str {
        match self {
            Buffer::Room(b) => b.title(),
            Buffer::Help(b) => b.title(),
        }
    }

    pub fn kind(&self) -> BufferKind {
        match self {
            Buffer::Room(b) => b.kind(),
            Buffer::Help(_) => BufferKind::Help,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        match self {
            Buffer::Room(b) => b.render(frame, area, theme),
            Buffer::Help(b) => b.render(frame, area, theme),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match self {
            Buffer::Room(b) => b.handle_key(key),
            Buffer::Help(b) => b.handle_key(key),
        }
    }
}
```

- [ ] **Step 2: Create `buffers/help.rs`**

```rust
//! HelpBuffer — static keybindings reference. Demonstrates that the Buffer
//! enum supports >1 variant (proves the abstraction is real).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::Theme;

const KEYBINDINGS: &[(&str, &str)] = &[
    // Global (Normal mode)
    ("Tab",       "next tab"),
    ("Shift-Tab", "previous tab"),
    ("Ctrl+B",   "toggle sidebar"),
    ("i",         "enter Insert mode (focus input)"),
    ("?",         "open this help"),
    ("q",         "quit"),
    ("Ctrl+C",   "quit"),
    // Insert mode
    ("Esc",       "Insert → Normal (input preserved)"),
    ("Enter",     "send message"),
    ("Backspace", "delete char"),
    // Buffer-local (Normal mode)
    ("PageUp/PageDown", "scroll timeline (Room)"),
    ("Home/End",       "top/bottom (Room)"),
    ("j/k or Up/Down", "scroll help text"),
];

pub struct HelpBuffer {
    pub scroll: u16,
}

impl HelpBuffer {
    #[must_use]
    pub fn new() -> Self {
        Self { scroll: 0 }
    }

    pub fn title(&self) -> &str {
        "? help"
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "Spaze · Phase 1.B keybindings",
            Style::default().fg(theme.foreground).add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::raw(""));
        for (key, desc) in KEYBINDINGS {
            lines.push(Line::from(vec![
                Span::styled(format!("  {key:>16}  "), Style::default().fg(theme.primary)),
                Span::styled(desc.to_string(), Style::default().fg(theme.muted)),
            ]));
        }
        let para = Paragraph::new(lines)
            .scroll((self.scroll, 0))
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(para, area);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_add(1);
                true
            }
            KeyCode::Home => {
                self.scroll = 0;
                true
            }
            _ => false,
        }
    }
}

impl Default for HelpBuffer {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 3: Create `buffers/room_timeline.rs` skeleton**

Skeleton — full render lands in Task 9. `push_message` and `handle_key` are real now.

```rust
//! RoomTimelineBuffer — chat timeline for a single Room (or DM).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use spaze_proto::{Message, RoomId};

use crate::buffers::BufferKind;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomKind {
    Standard,
    Direct,
}

#[derive(Debug, Clone)]
pub struct RepoBinding {
    pub repo_name: String,
    pub subpath: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollState {
    pub offset_from_bottom: u16,
    pub stuck_to_bottom: bool,
}

impl ScrollState {
    #[must_use]
    pub fn new() -> Self {
        Self { offset_from_bottom: 0, stuck_to_bottom: true }
    }

    pub fn page_up(&mut self) {
        self.offset_from_bottom = self.offset_from_bottom.saturating_add(10);
        self.stuck_to_bottom = false;
    }

    pub fn page_down(&mut self) {
        self.offset_from_bottom = self.offset_from_bottom.saturating_sub(10);
        if self.offset_from_bottom == 0 {
            self.stuck_to_bottom = true;
        }
    }

    pub fn to_bottom(&mut self) {
        self.offset_from_bottom = 0;
        self.stuck_to_bottom = true;
    }

    pub fn to_top(&mut self) {
        self.offset_from_bottom = u16::MAX;
        self.stuck_to_bottom = false;
    }

    pub fn stick_to_bottom_if_was(&mut self) {
        if self.stuck_to_bottom {
            self.offset_from_bottom = 0;
        }
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RoomTimelineBuffer {
    pub room_id: RoomId,
    pub display_name: String,
    pub kind: RoomKind,
    pub repo_binding: Option<RepoBinding>,
    pub topic: Option<String>,
    pub messages: Vec<Message>,
    pub scroll: ScrollState,
    /// Used for mention detection in render. Set from Identity at construction.
    pub self_display_name: String,
}

impl RoomTimelineBuffer {
    #[must_use]
    pub fn new(room_id: RoomId, display_name: String, kind: RoomKind, self_display_name: String) -> Self {
        Self {
            room_id,
            display_name,
            kind,
            repo_binding: None,
            topic: None,
            messages: Vec::new(),
            scroll: ScrollState::new(),
            self_display_name,
        }
    }

    pub fn push_message(&mut self, msg: Message) {
        self.messages.push(msg);
        self.scroll.stick_to_bottom_if_was();
    }

    pub fn title(&self) -> &str {
        &self.display_name
    }

    pub fn kind(&self) -> BufferKind {
        match self.kind {
            RoomKind::Standard => BufferKind::Room,
            RoomKind::Direct => BufferKind::DirectMessage,
        }
    }

    /// Real implementation lands in Task 9.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, _theme: &Theme) {
        let placeholder = Paragraph::new(format!(
            "[room timeline placeholder — {} messages]",
            self.messages.len()
        ));
        frame.render_widget(placeholder, area);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::PageUp => {
                self.scroll.page_up();
                true
            }
            KeyCode::PageDown => {
                self.scroll.page_down();
                true
            }
            KeyCode::Home => {
                self.scroll.to_top();
                true
            }
            KeyCode::End => {
                self.scroll.to_bottom();
                true
            }
            _ => false,
        }
    }
}
```

- [ ] **Step 4: Add `pub mod buffers;` to `lib.rs`**

In `/Users/at-a/Repos/Spaze/spaze-client/src/lib.rs`, near the existing `pub mod identity;`, add:

```rust
pub mod buffers;
```

- [ ] **Step 5: Verify compilation**

```bash
cargo check -p spaze-client
```

Expected: clean compile.

- [ ] **Step 6: Run lint + fmt + tests**

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test --workspace --all-targets
```

Expected: 25 tests pass (no new tests yet — those land in Task 12).

- [ ] **Step 7: Commit**

```bash
git add spaze-client/src/buffers/ spaze-client/src/lib.rs
git commit -m "feat(client): Buffer enum + HelpBuffer + RoomTimelineBuffer skeleton" -m "Buffer is an enum (not Box<dyn Trait>) — compile-time exhaustiveness, no dynamic dispatch overhead. Two variants in 1.B; Phase 5+ adds Pins/Files/Notez/Todos/Users/Settings. RoomTimelineBuffer holds messages and scroll state; render is a placeholder (Task 9 fills in real rendering)."
```

---

## Task 6: `App` state machine

**Files:**
- Create: `/Users/at-a/Repos/Spaze/spaze-client/src/app.rs`
- Modify: `/Users/at-a/Repos/Spaze/spaze-client/src/lib.rs` (`pub mod app;`)

- [ ] **Step 1: Create `spaze-client/src/app.rs`**

```rust
//! App state machine — top-level state for the TUI client.

use spaze_proto::{DeviceId, RoomId, UserId};
use uuid::Uuid;

use crate::buffers::{Buffer, HelpBuffer, RoomTimelineBuffer, room_timeline::RoomKind};
use crate::theme::{Theme, catppuccin_mocha::CATPPUCCIN_MOCHA};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Insert,
}

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Connecting,
    Connected { server_url: String },
    Disconnected { reason: String },
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub user_id: UserId,
    pub device_id: DeviceId,
    pub display_name: String,
}

pub struct App {
    pub buffers: Vec<Buffer>,
    pub active: usize,
    pub sidebar_visible: bool,
    pub mode: InputMode,
    pub input_buffer: String,
    pub theme: Theme,
    pub connection: ConnectionState,
    pub identity: Identity,
    pub should_quit: bool,
}

impl App {
    /// Construct initial state with the hardcoded room and a help buffer.
    #[must_use]
    pub fn new(identity: Identity, server_url: String) -> Self {
        let room_id = RoomId::from_uuid(Uuid::nil());
        let room = RoomTimelineBuffer::new(
            room_id,
            "# general".to_string(),
            RoomKind::Standard,
            identity.display_name.clone(),
        );
        let help = HelpBuffer::new();
        Self {
            buffers: vec![Buffer::Room(room), Buffer::Help(help)],
            active: 0,
            sidebar_visible: true,
            mode: InputMode::Normal,
            input_buffer: String::new(),
            theme: CATPPUCCIN_MOCHA,
            connection: ConnectionState::Connecting,
            identity,
            should_quit: false,
        }
    }

    /// Cycle to the next tab (wraps around).
    pub fn cycle_tab_forward(&mut self) {
        self.active = (self.active + 1) % self.buffers.len();
    }

    /// Cycle to the previous tab (wraps around).
    pub fn cycle_tab_backward(&mut self) {
        if self.active == 0 {
            self.active = self.buffers.len() - 1;
        } else {
            self.active -= 1;
        }
    }

    /// Toggle the sidebar.
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
    }

    /// Find the index of a buffer by predicate.
    pub fn find_buffer<F: Fn(&Buffer) -> bool>(&self, pred: F) -> Option<usize> {
        self.buffers.iter().position(|b| pred(b))
    }
}
```

- [ ] **Step 2: Add `pub mod app;` to `lib.rs`**

```rust
pub mod app;
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p spaze-client
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Expected: all clean.

- [ ] **Step 4: Commit**

```bash
git add spaze-client/src/app.rs spaze-client/src/lib.rs
git commit -m "feat(client): App state machine with InputMode/ConnectionState/Identity" -m "Initial state: room + help buffer, sidebar visible, Normal mode, Catppuccin Mocha theme. Cycle helpers (cycle_tab_forward/backward) and toggle_sidebar are tested in Task 12."
```

---

## Task 7: TUI module skeleton — setup, teardown, draw, min size

**Files:**
- Create: `/Users/at-a/Repos/Spaze/spaze-client/src/tui.rs`
- Modify: `/Users/at-a/Repos/Spaze/spaze-client/src/lib.rs` (`pub mod tui;`)

- [ ] **Step 1: Create `spaze-client/src/tui.rs` skeleton**

```rust
//! TUI module — terminal setup/teardown and the top-level draw function.
//!
//! `draw()` slices the screen into 7 named regions and dispatches to per-region
//! helpers (`draw_topbar`, `draw_sidebar`, etc.). Each helper takes a Rect and
//! a `&Theme`; rendering is theme-driven throughout.
//!
//! Phase 1.B uses placeholder content for most regions — they get real
//! rendering in Task 8. The structure is in place from this task.

use std::io;

use anyhow::{Context, Result};
use crossterm::ExecutableCommand;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub const MIN_COLS: u16 = 60;
pub const MIN_ROWS: u16 = 20;

pub type Tui = Terminal<CrosstermBackend<io::Stdout>>;

/// Enter raw mode + alt screen. Returns a Terminal handle.
///
/// # Errors
///
/// Returns an error if the terminal can't be put into raw mode.
pub fn setup() -> Result<Tui> {
    enable_raw_mode().context("enable_raw_mode")?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen).context("EnterAlternateScreen")?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).context("Terminal::new")?;
    Ok(terminal)
}

/// Leave raw mode + alt screen.
///
/// # Errors
///
/// Returns an error if the terminal can't be restored. Tries to do as much
/// cleanup as possible regardless.
pub fn teardown(mut terminal: Tui) -> Result<()> {
    let mut stdout = io::stdout();
    let _ = stdout.execute(DisableMouseCapture);
    let _ = stdout.execute(LeaveAlternateScreen);
    let _ = disable_raw_mode();
    let _ = terminal.show_cursor();
    Ok(())
}

/// Draw one frame given the current app state.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Floor check.
    let need_sidebar = app.sidebar_visible;
    let effective_min_cols = if need_sidebar { MIN_COLS } else { 50 };
    if area.width < effective_min_cols || area.height < MIN_ROWS {
        draw_too_small(frame, area, &app.theme);
        return;
    }

    // Vertical: topbar (1) | middle (flex) | statusbar (1).
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(area);
    let top = outer[0];
    let middle = outer[1];
    let bottom = outer[2];

    // Horizontal in middle: optional sidebar | main.
    let (sidebar_area, main_area) = if app.sidebar_visible {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(24), Constraint::Min(0)])
            .split(middle);
        (Some(split[0]), split[1])
    } else {
        (None, middle)
    };

    // Vertical in main: tabs (auto, 1-3) | header (0 or 1) | buffer (flex) | input (1-3).
    let active_kind = app.buffers[app.active].kind();
    let needs_room_header = matches!(
        active_kind,
        crate::buffers::BufferKind::Room | crate::buffers::BufferKind::DirectMessage
    );
    let main_constraints: Vec<Constraint> = if needs_room_header {
        vec![Constraint::Length(2), Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)]
    } else {
        vec![Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)]
    };
    let main_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints(main_constraints)
        .split(main_area);

    let tabs_area = main_split[0];
    let (header_area, buffer_area, input_area) = if needs_room_header {
        (Some(main_split[1]), main_split[2], main_split[3])
    } else {
        (None, main_split[1], main_split[2])
    };

    draw_topbar(frame, top, app, &app.theme.clone());
    if let Some(sa) = sidebar_area {
        draw_sidebar(frame, sa, app);
    }
    draw_tab_strip(frame, tabs_area, app);
    if let Some(ha) = header_area {
        draw_room_header(frame, ha, app);
    }
    let theme = app.theme;
    let active_buffer = &mut app.buffers[app.active];
    active_buffer.render(frame, buffer_area, &theme);
    draw_input(frame, input_area, app);
    draw_statusbar(frame, bottom, app);
}

fn draw_too_small(frame: &mut Frame, area: Rect, theme: &crate::theme::Theme) {
    let msg = format!("terminal too small (need {MIN_COLS}×{MIN_ROWS})");
    let para = Paragraph::new(msg)
        .style(Style::default().fg(theme.error))
        .block(Block::default().borders(Borders::NONE));
    // Center-ish placement
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1), Constraint::Min(0)])
        .split(area);
    frame.render_widget(para, inner[1]);
}

// Per-region helpers — placeholder bodies; Task 8 fills these in.

fn draw_topbar(frame: &mut Frame, area: Rect, app: &App, theme: &crate::theme::Theme) {
    let _ = (frame, area, app, theme);
}

fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let _ = (frame, area, app);
}

fn draw_tab_strip(frame: &mut Frame, area: Rect, app: &App) {
    let _ = (frame, area, app);
}

fn draw_room_header(frame: &mut Frame, area: Rect, app: &App) {
    let _ = (frame, area, app);
}

fn draw_input(frame: &mut Frame, area: Rect, app: &App) {
    let _ = (frame, area, app);
}

fn draw_statusbar(frame: &mut Frame, area: Rect, app: &App) {
    let _ = (frame, area, app);
}
```

- [ ] **Step 2: Add `pub mod tui;` to `lib.rs`**

```rust
pub mod tui;
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p spaze-client
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Expected: clean. Some unused-variable warnings will appear in the placeholder bodies — those are intentional via the `let _ = ...` pattern; clippy should accept this.

- [ ] **Step 4: Commit**

```bash
git add spaze-client/src/tui.rs spaze-client/src/lib.rs
git commit -m "feat(client): TUI module with setup/teardown/draw skeleton + min size" -m "draw() slices screen into 7 regions (topbar/sidebar/tab_strip/room_header/buffer/input/statusbar). Per-region helpers are placeholders; Task 8 fills them in. Minimum size enforced (60×20 with sidebar, 50×20 without)."
```

---

## Task 8: Region rendering helpers — the real implementations

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/spaze-client/src/tui.rs`

This task fills in the 6 placeholder helpers from Task 7 with real rendering.

- [ ] **Step 1: Implement `draw_topbar`**

Replace the placeholder body:

```rust
fn draw_topbar(frame: &mut Frame, area: Rect, app: &App, theme: &crate::theme::Theme) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};

    let short = format!("{:.8}", app.identity.user_id.as_uuid().simple());
    let conn_text = match &app.connection {
        crate::app::ConnectionState::Connecting => Span::styled(
            "  connecting…  ",
            Style::default().fg(theme.warning),
        ),
        crate::app::ConnectionState::Connected { server_url } => Span::styled(
            format!("  connected  {server_url}  "),
            Style::default().fg(theme.success),
        ),
        crate::app::ConnectionState::Disconnected { reason } => Span::styled(
            format!("  disconnected: {reason}  "),
            Style::default().fg(theme.error),
        ),
    };

    let line = Line::from(vec![
        Span::styled(
            format!(" spaze · {}@{}  ", app.identity.display_name, short),
            Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
        ),
        conn_text,
    ]);

    let para = Paragraph::new(line)
        .style(Style::default().bg(theme.surface).fg(theme.foreground));
    frame.render_widget(para, area);
}
```

- [ ] **Step 2: Implement `draw_sidebar`**

```rust
fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};

    let theme = &app.theme;
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![Span::styled(
        " SERVERS",
        Style::default().fg(theme.muted),
    )]));

    // 1.B: one hardcoded server (the one we're connected to).
    let server_label = match &app.connection {
        crate::app::ConnectionState::Connected { server_url } => {
            // Compact: strip ws:// prefix for display.
            server_url.trim_start_matches("ws://").to_string()
        }
        _ => "—".to_string(),
    };
    lines.push(Line::from(vec![Span::styled(
        format!(" ▼ {server_label}"),
        Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "    › repo binding TBD".to_string(),
        Style::default().fg(theme.muted),
    )]));

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![Span::styled(
        "    ROOMS",
        Style::default().fg(theme.muted),
    )]));

    // List each Room buffer.
    for (i, buf) in app.buffers.iter().enumerate() {
        if let crate::buffers::Buffer::Room(rb) = buf {
            let active = i == app.active;
            let style = if active {
                Style::default().fg(theme.tab_room).bg(theme.overlay)
            } else {
                Style::default().fg(theme.subtle)
            };
            lines.push(Line::from(vec![Span::styled(
                format!("    {}", rb.title()),
                style,
            )]));
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![Span::styled(
        "    DIRECT MESSAGES",
        Style::default().fg(theme.muted),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "    (none)".to_string(),
        Style::default().fg(theme.subtle),
    )]));

    // Footer icons strip.
    let mut footer_y = area.height.saturating_sub(2);
    if footer_y < lines.len() as u16 {
        // Push padding to avoid overlap if sidebar is tall.
        footer_y = lines.len() as u16 + 1;
    }
    while (lines.len() as u16) < footer_y {
        lines.push(Line::raw(""));
    }
    lines.push(Line::from(vec![Span::styled(
        " + add server…",
        Style::default().fg(theme.muted),
    )]));
    lines.push(Line::from(vec![Span::styled(
        " 🔍 search   ⚙ settings   ? help",
        Style::default().fg(theme.muted),
    )]));

    let para = Paragraph::new(lines)
        .style(Style::default().bg(theme.surface));
    frame.render_widget(para, area);
}
```

- [ ] **Step 3: Implement `draw_tab_strip`**

```rust
fn draw_tab_strip(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};

    let theme = &app.theme;
    let mut current_line: Vec<Span> = Vec::new();
    let mut current_width: u16 = 0;
    let mut lines: Vec<Line> = Vec::new();
    const GUTTER: u16 = 1;

    for (i, buf) in app.buffers.iter().enumerate() {
        let active = i == app.active;
        let label = format!(" {} ", buf.title());
        let label_w = label.chars().count() as u16;
        let kind = buf.kind();
        let fg = match kind {
            crate::buffers::BufferKind::Room => theme.tab_room,
            crate::buffers::BufferKind::DirectMessage => theme.tab_dm,
            crate::buffers::BufferKind::Help => theme.muted,
            _ => theme.tab_special,
        };
        let style = if active {
            Style::default().fg(fg).bg(theme.overlay).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(fg)
        };

        if current_width + label_w + GUTTER > area.width && !current_line.is_empty() {
            lines.push(Line::from(std::mem::take(&mut current_line)));
            current_width = 0;
        }
        current_line.push(Span::styled(label, style));
        current_line.push(Span::raw(" "));
        current_width += label_w + GUTTER;

        // Cap at 3 rows.
        if lines.len() >= 3 {
            // Drop overflow with ellipsis on last line.
            current_line = vec![Span::styled(" … ", Style::default().fg(theme.muted))];
            break;
        }
    }
    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    let para = Paragraph::new(lines).style(Style::default().bg(theme.surface));
    frame.render_widget(para, area);
}
```

- [ ] **Step 4: Implement `draw_room_header`**

```rust
fn draw_room_header(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};

    let theme = &app.theme;
    let active_buf = &app.buffers[app.active];
    let crate::buffers::Buffer::Room(rb) = active_buf else {
        return;
    };

    let mut spans = vec![Span::styled(
        format!(" {} ", rb.title()),
        Style::default().fg(theme.tab_room).add_modifier(Modifier::BOLD),
    )];
    if let Some(binding) = &rb.repo_binding {
        let binding_text = match &binding.subpath {
            Some(p) => format!("· {}/", p),
            None => format!("· {}", binding.repo_name),
        };
        spans.push(Span::styled(binding_text, Style::default().fg(theme.tab_special)));
    }
    if let Some(t) = &rb.topic {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("— {t}"),
            Style::default().fg(theme.muted),
        ));
    }

    let para = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(theme.surface));
    frame.render_widget(para, area);
}
```

- [ ] **Step 5: Implement `draw_input`**

```rust
fn draw_input(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};

    let theme = &app.theme;
    let prompt = match app.mode {
        crate::app::InputMode::Normal => Span::styled(
            "› ",
            Style::default().fg(theme.muted),
        ),
        crate::app::InputMode::Insert => Span::styled(
            "› ",
            Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
        ),
    };
    let body = Span::styled(
        app.input_buffer.clone(),
        Style::default().fg(theme.foreground),
    );
    let cursor = if matches!(app.mode, crate::app::InputMode::Insert) {
        Span::styled("_", Style::default().fg(theme.primary).add_modifier(Modifier::SLOW_BLINK))
    } else {
        Span::raw("")
    };

    let line = Line::from(vec![prompt, body, cursor]);
    let para = Paragraph::new(line)
        .style(Style::default().bg(theme.surface));
    frame.render_widget(para, area);
}
```

- [ ] **Step 6: Implement `draw_statusbar`**

```rust
fn draw_statusbar(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};

    let theme = &app.theme;
    let mode_text = match app.mode {
        crate::app::InputMode::Normal => "NORMAL",
        crate::app::InputMode::Insert => "INSERT",
    };
    let active_title = app.buffers[app.active].title().to_string();
    let msg_count = match &app.buffers[app.active] {
        crate::buffers::Buffer::Room(rb) => rb.messages.len(),
        crate::buffers::Buffer::Help(_) => 0,
    };

    let left = Line::from(vec![
        Span::styled(
            format!(" {mode_text} "),
            Style::default().fg(theme.background).bg(theme.primary).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("· {active_title} · {msg_count} messages"),
            Style::default().fg(theme.muted),
        ),
    ]);

    let right = Line::from(vec![Span::styled(
        format!("theme: {} ", theme.syntect_theme_name),
        Style::default().fg(theme.muted),
    )]);

    let parts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(40)])
        .split(area);
    frame.render_widget(Paragraph::new(left).style(Style::default().bg(theme.surface)), parts[0]);
    frame.render_widget(Paragraph::new(right).style(Style::default().bg(theme.surface)), parts[1]);
}
```

- [ ] **Step 7: Verify compilation + lints**

```bash
cargo check -p spaze-client
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add spaze-client/src/tui.rs
git commit -m "feat(client): real region renderers (topbar/sidebar/tabs/header/input/status)" -m "Each region pulls colors from the active theme. Tab strip auto-wraps to 3 rows, ellipses overflow. Sidebar shows the connected server + room buffers + DM placeholder."
```

---

## Task 9: Real `RoomTimelineBuffer::render` + `HelpBuffer` polish

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/spaze-client/src/buffers/room_timeline.rs`

The `HelpBuffer::render` from Task 5 is already real. This task replaces the placeholder `RoomTimelineBuffer::render` with the full implementation.

- [ ] **Step 1: Replace `RoomTimelineBuffer::render`**

Find the placeholder:

```rust
pub fn render(&mut self, frame: &mut Frame, area: Rect, _theme: &Theme) {
    let placeholder = Paragraph::new(format!(
        "[room timeline placeholder — {} messages]",
        self.messages.len()
    ));
    frame.render_widget(placeholder, area);
}
```

Replace with:

```rust
pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
    use chrono::TimeZone;
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use spaze_proto::MessageBody;

    let mut lines: Vec<Line> = Vec::with_capacity(self.messages.len());

    for msg in &self.messages {
        let ts = chrono::Utc
            .timestamp_millis_opt(msg.created_at_ms)
            .single()
            .map(|dt| dt.format("%H:%M").to_string())
            .unwrap_or_else(|| "??:??".into());

        let author_display = if msg.author_display_name.is_empty() {
            // Fallback to 8-char hex prefix (same shape as 1.A's stdout output).
            format!("{:.8}", msg.author_id.as_uuid().simple())
        } else {
            msg.author_display_name.clone()
        };

        let mention = !self.self_display_name.is_empty()
            && body_text(&msg.body)
                .to_lowercase()
                .contains(&format!("@{}", self.self_display_name).to_lowercase());

        let line_style = if mention {
            Style::default().bg(theme.mention)
        } else {
            Style::default()
        };

        let mut spans = vec![
            Span::styled(format!("{ts} "), Style::default().fg(theme.muted)),
            Span::styled(
                format!("<{author_display}> "),
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ),
        ];
        match &msg.body {
            MessageBody::Text { content } => {
                spans.push(Span::styled(content.clone(), Style::default().fg(theme.foreground)));
            }
            MessageBody::System { content } => {
                spans.push(Span::styled(
                    format!("-- system: {content}"),
                    Style::default().fg(theme.subtle).add_modifier(Modifier::ITALIC),
                ));
            }
        }
        if msg.deleted_at_ms.is_some() {
            spans.push(Span::styled(
                "  [deleted]".to_string(),
                Style::default().fg(theme.subtle).add_modifier(Modifier::ITALIC),
            ));
        }
        if msg.edited_at_ms.is_some() {
            spans.push(Span::styled(
                "  (edited)".to_string(),
                Style::default().fg(theme.muted),
            ));
        }
        lines.push(Line::from(spans).style(line_style));
    }

    let para = ratatui::widgets::Paragraph::new(lines)
        .scroll((self.scroll.offset_from_bottom, 0))
        .style(Style::default().fg(theme.foreground).bg(theme.background));
    frame.render_widget(para, area);
}

fn body_text(body: &spaze_proto::MessageBody) -> &str {
    match body {
        spaze_proto::MessageBody::Text { content } => content,
        spaze_proto::MessageBody::System { content } => content,
    }
}
```

- [ ] **Step 2: Add `chrono` to workspace deps**

`chrono` is needed for timestamp formatting. Add to root `Cargo.toml` `[workspace.dependencies]`:

```toml
chrono = { version = "0.4", default-features = false, features = ["clock"] }
```

And to `spaze-client/Cargo.toml` `[dependencies]`:

```toml
chrono.workspace = true
```

Verify it resolves:

```bash
cargo check -p spaze-client
```

- [ ] **Step 3: Run workspace verification**

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test --workspace --all-targets
```

Expected: 25 tests pass.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock spaze-client/Cargo.toml spaze-client/src/buffers/room_timeline.rs
git commit -m "feat(client): real RoomTimelineBuffer::render with timestamps + mentions" -m "Renders each message as 'HH:MM <author> body' using theme slots. Mention detection (case-insensitive @<self.display_name>) applies the mention background. Deleted/edited markers render inline. chrono added to workspace deps for timestamp formatting."
```

---

## Task 10: Main loop integration — crossterm + WS + ctrl_c

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/spaze-client/src/lib.rs` (replace stdin loop with TUI loop)
- Delete: `/Users/at-a/Repos/Spaze/spaze-client/src/render.rs`

This is the load-bearing integration — replaces the 1.A stdin loop with the TUI event loop.

- [ ] **Step 1: Delete the now-obsolete `render.rs`**

```bash
git rm spaze-client/src/render.rs
```

Its `print_message` / `render_frame` functions are obsoleted by `RoomTimelineBuffer::render`.

- [ ] **Step 2: Rewrite `spaze-client/src/lib.rs`**

Replace the existing file with:

```rust
//! `spaze-client` — TUI client for the Spaze chat protocol.
//!
//! Phase 1.B: ratatui+crossterm TUI replacing 1.A's stdin/stdout. Two real
//! buffer kinds (Room, Help). Catppuccin Mocha theme.

use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures_util::{SinkExt, StreamExt};
use spaze_proto::{
    ClientCommand, ClientFrame, CommandOutcome, DeviceId, MessageBody, RequestId,
    ResponsePayload, RoomId, ServerEvent, ServerFrame, UserId,
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

/// Run the TUI client until Ctrl+C / stdin EOF / server close.
///
/// # Errors
///
/// Returns an error on connection failure or unrecoverable WS error.
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
    app.connection = ConnectionState::Connected { server_url: config.server_url.clone() };

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
                        Some(Ok(CrosstermEvent::Resize(_, _))) => {
                            // re-draw at end of loop
                        }
                        Some(Ok(_)) => {} // mouse, paste, focus — ignored in 1.B
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
        let _ = sink.send(WsMessage::Close(None)).await;
        Ok(())
    }.await;

    tui::teardown(terminal)?;

    // Print disconnect reason after teardown so stderr is back to normal.
    if let ConnectionState::Disconnected { reason } = &app.connection {
        eprintln!("connection lost: {reason}");
    }

    result
}

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
                            body: MessageBody::Text { content: app.input_buffer.clone() },
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
        (KeyCode::Char('c'), true) => {
            app.should_quit = true;
            return Ok(());
        }
        (KeyCode::Char('b'), true) => {
            app.toggle_sidebar();
            return Ok(());
        }
        (KeyCode::Char('q'), false) => {
            app.should_quit = true;
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
        ServerFrame::Response { result: CommandOutcome::Ok { payload: ResponsePayload::MessagePosted(m) }, .. } => {
            push_to_room(app, m);
        }
        ServerFrame::Response { result: CommandOutcome::Ok { payload: ResponsePayload::Empty }, .. } => {}
        ServerFrame::Response { result: CommandOutcome::Err { error }, .. } => {
            warn!("server response error: {error}");
        }
        ServerFrame::Event(ServerEvent::MessagePosted(m)) => {
            push_to_room(app, m);
        }
        ServerFrame::Event(_) => {} // 1.A doesn't emit other events
    }
}

fn push_to_room(app: &mut App, msg: spaze_proto::Message) {
    // 1.B: one room. Find it and push.
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
```

- [ ] **Step 3: Update `main.rs` to pass display_name**

Find the `ClientConfig` construction in `/Users/at-a/Repos/Spaze/spaze-client/src/main.rs`. The current shape is missing `display_name`. Update:

```rust
let config = ClientConfig {
    server_url: cli.server,
    user_id,
    device_id,
    display_name: name,
    room_id,
};
```

Make sure `name` is in scope (the resolved `--name`/`$USER`/`anon` value). The previous code was:

```rust
let name = cli
    .name
    .unwrap_or_else(|| std::env::var("USER").unwrap_or_else(|_| "anon".to_string()));
let (user_id, device_id) = derive_identity(&name);
```

Then change the `ClientConfig { ... }` to include `display_name: name`. Note this CONSUMES `name`, which is fine since it's the last use.

Full `main.rs` after the change:

```rust
use anyhow::Result;
use clap::Parser;
use spaze_client::{ClientConfig, identity::derive_identity, run};
use spaze_proto::RoomId;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(version, about = "Spaze TUI client (Phase 1.B: ratatui)")]
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

    let room_id = RoomId::from_uuid(Uuid::nil());

    let config = ClientConfig {
        server_url: cli.server,
        user_id,
        device_id,
        display_name: name,
        room_id,
    };

    run(config).await
}
```

- [ ] **Step 4: Update the integration test**

The integration test in `tests/two_client_chat.rs` doesn't call `spaze_client::run` (it uses raw WS connections), but we DO need to make sure `ClientConfig`'s shape change doesn't break anything. It doesn't directly construct `ClientConfig` so should be fine. Verify with cargo check:

```bash
cargo check --workspace --all-targets
```

Expected: clean.

- [ ] **Step 5: Run lint + fmt + tests**

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test --workspace --all-targets
```

Expected: 25 tests pass. (No new tests yet.)

- [ ] **Step 6: Commit**

```bash
git add -A spaze-client/src/
git commit -m "feat(client): TUI main loop replaces stdin (crossterm + ws + ctrl_c)" -m "spaze-client/src/render.rs deleted (logic moved to RoomTimelineBuffer). Main loop is tokio::select! over crossterm events / ws frames / ctrl_c, with a redraw after each event. Modal input (Normal/Insert), global keybindings (Tab, Ctrl+B, ?, q, i), Insert handles printable chars + Enter (sends) + Backspace + Esc."
```

---

## Task 11: Manual smoke test

**Files:** none (manual verification step)

- [ ] **Step 1: Build all binaries**

```bash
cargo build --bin spaze --bin spaze-server
```

Expected: both build cleanly.

- [ ] **Step 2: Run the three-terminal demo**

In **Terminal 1**: `cargo run --bin spaze-server`. Expected: prints `listening on 127.0.0.1:9876`.

In **Terminal 2**: `cargo run --bin spaze -- --name andreas`. Expected: alt-screen activated; TUI renders with sidebar showing `127.0.0.1:9876`, `# general` room visible; tab strip showing `# general` and `? help`; status bar showing `NORMAL`, `theme: Catppuccin Mocha`.

In **Terminal 3**: `cargo run --bin spaze -- --name beth`. Same — TUI renders, both clients in `# general`.

- [ ] **Step 3: Send a message**

In Terminal 2: press `i` (Normal → Insert). Type "hello world". Press Enter. Expected:
- Terminal 2's input clears.
- Terminal 2's timeline shows the message: `HH:MM <andreas> hello world`.
- Terminal 3's timeline shows the same line.

In Terminal 3: press `i`, type "hi back!", Enter. Expected:
- Same shape — message appears in both terminals with `<beth>` prefix.

- [ ] **Step 4: Mention detection**

In Terminal 2: press `i`, type "@beth got it", Enter. Expected:
- Terminal 3 (where the recipient is "beth"): the line has the mention background color (pink-ish).
- Terminal 2 (sender): no mention highlight (not addressed to self).

- [ ] **Step 5: Tab cycling + help**

In either client: press `?`. Expected: tab strip's `? help` becomes active. Buffer area shows the keybindings reference.

Press `Tab`. Expected: cycles back to the room. Press `Tab` again — back to help.

Press `Shift-Tab`. Expected: cycles backward.

- [ ] **Step 6: Sidebar toggle**

Press `Ctrl+B`. Expected: sidebar disappears; main area expands to fill. Press `Ctrl+B` again — sidebar returns.

- [ ] **Step 7: Resize**

Resize the terminal window narrower than 60 columns (sidebar visible). Expected: "terminal too small (need 60×20)" message centered. Resize back; layout restores.

Hide the sidebar (Ctrl+B), then resize to 50–59 columns. Expected: layout works (effective floor is 50 cols without sidebar).

- [ ] **Step 8: Modal navigation**

In a client: press `i` (enter Insert). Type some text. Press `Esc`. Expected: mode bar shows `NORMAL`; input buffer preserved (text still visible). Press `i` again, finish typing, Enter. Message sends.

- [ ] **Step 9: Ctrl+C on client**

Press `Ctrl+C` in a client. Expected: terminal mode restored cleanly (no garbled output, alt-screen exited). Exit code 0. Server keeps running.

- [ ] **Step 10: Ctrl+C on server**

Stop the server with Ctrl+C. Expected: remaining clients show "connection lost: server closed" briefly in topbar/statusbar, then exit gracefully (terminal restored).

If steps 1–10 all pass, 1.B's manual deliverable is complete.

- [ ] **Step 11: Commit a marker if any small issues were fixed**

If steps 1–10 surface bugs, fix them and add commits. Otherwise no commit for this task.

---

## Task 12: Unit tests

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/spaze-client/src/lib.rs` (add 7 more unit tests; theme test already exists)

- [ ] **Step 1: Add the 7 new unit tests**

Append to the `tests` module in `/Users/at-a/Repos/Spaze/spaze-client/src/lib.rs` (the existing tests stay):

```rust
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
    use spaze_proto::{
        DeviceId, Message, MessageBody, MessageId, RoomId, UserId,
    };

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
        body: MessageBody::Text { content: "hi".into() },
    };
    rb.push_message(msg);
    assert_eq!(rb.messages.len(), 1);
}

#[test]
fn room_buffer_scroll_sticks_to_bottom_on_new_message() {
    use super::buffers::room_timeline::{RoomKind, RoomTimelineBuffer};
    use spaze_proto::{
        DeviceId, Message, MessageBody, MessageId, RoomId, UserId,
    };

    let mut rb = RoomTimelineBuffer::new(
        RoomId::new(),
        "# test".to_string(),
        RoomKind::Standard,
        "self".to_string(),
    );
    let make_msg = |content: &str| Message {
        id: MessageId::new(),
        room_id: rb.room_id,
        author_id: UserId::new(),
        author_device_id: DeviceId::new(),
        author_display_name: "alice".to_string(),
        created_at_ms: 0,
        edited_at_ms: None,
        deleted_at_ms: None,
        body: MessageBody::Text { content: content.into() },
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
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use super::buffers::HelpBuffer;

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
    assert!("Hello @andreas, how's it going".to_lowercase().contains(&needle));
    assert!(!"hello world".to_lowercase().contains(&needle));
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p spaze-client
```

Expected: 12 tests pass total in spaze-client (4 identity + 1 theme + 7 new = 12). Workspace total: 32 tests (16 proto + 4 server + 12 client).

- [ ] **Step 3: Run lint + fmt**

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add spaze-client/src/lib.rs
git commit -m "test(client): unit tests for App, buffers, scroll, mode transitions" -m "7 new tests covering: app init shape, push_message append, scroll-stick-to-bottom invariant, tab cycle wrap, help buffer scroll, mode transition input preservation, mention detection."
```

---

## Task 13: Update DESIGN.md and README.md

**Files:**
- Modify: `/Users/at-a/Repos/Spaze/DESIGN.md`
- Modify: `/Users/at-a/Repos/Spaze/README.md`

- [ ] **Step 1: Collapse the three-level hierarchy table in DESIGN.md**

Find the table under `## Three-level hierarchy` heading. Replace the heading and table:

```markdown
## Three-level hierarchy

| Level | What it is | Identity | Wire-level type |
|-------|-----------|----------|-----------------|
| **Server** | A running `spaze-server` daemon (one machine, one URL) | hostname + port | not a wire ID — connect by URL |
| **Space** | A project workspace inside a server (multi-Space per server supported) | UUIDv7 | `SpaceId` |
| **Room** | A conversation channel inside a Space, optionally repo-bound | UUIDv7 | `RoomId` |
```

With:

```markdown
## Two-level hierarchy

| Level | What it is | Identity | Wire-level type |
|-------|-----------|----------|-----------------|
| **Server (= Space)** | A running `spaze-server` daemon — one URL = one project workspace | hostname + port | `SpaceId` (always 1-per-server in practice; retained as a wire type for forward compatibility) |
| **Room** | A conversation channel inside the server, optionally repo-bound | UUIDv7 | `RoomId` |

DMs are Rooms with `kind = direct` and exactly two members. UI presents them in a separate "Direct messages" subgroup beneath each server.

If a deployment wants two project workspaces, they run two `spaze-server` instances (Mastodon-style: instance == community). Multi-Space-per-server was considered and dropped — the protocol keeps `SpaceId` for future flexibility, but the UI / UX flattens to two levels.
```

- [ ] **Step 2: Update Phase 1 row in Phased Scope**

Find the Phase 1 row in the `## Phased Scope` table:

```markdown
| 1 | 1 | Workspace, `spaze-proto`, server + client skeletons, buffer abstraction, slash command parser, theming scaffold, single hardcoded Space/Room, plaintext WebSocket — two terminals chat. **Decomposed into 1.A (bare WS chat ✅), 1.B (TUI + buffer + theming), 1.C (slash commands).** |
```

Replace with:

```markdown
| 1 | 1 | Workspace, `spaze-proto`, server + client skeletons, buffer abstraction, slash command parser, theming scaffold, single hardcoded Space/Room, plaintext WebSocket — two terminals chat. **Decomposed: 1.A (bare WS chat ✅), 1.B (TUI + buffer + theming ✅), 1.C (slash commands).** |
```

- [ ] **Step 3: Update the sub-project tracking table**

Find the table under `## Sub-project tracking`:

```markdown
| 1.B — TUI shell + buffer + theming scaffold | not yet planned | — | — |
```

Replace with:

```markdown
| 1.B — TUI shell + buffer + theming scaffold | ✅ shipped | `2026-05-05-phase-1b-tui-shell-design.md` | `2026-05-05-phase-1b-tui-shell.md` |
```

- [ ] **Step 4: Update README.md status line**

Find:

```markdown
> **Status:** pre-alpha. Phase 1.A complete — bare WS chat works in two terminals. TUI, theming, slash commands, auth still ahead.
```

Replace with:

```markdown
> **Status:** pre-alpha. Phase 1.B complete — real TUI with Catppuccin Mocha theme, two terminals chat with author names. Slash commands and auth still ahead.
```

- [ ] **Step 5: Update README.md `## Running` section**

Find the `## Running (Phase 1.A bare-WS demo)` section. Replace the heading and prose:

```markdown
## Running (Phase 1.B bare-WS TUI demo)

Three terminals — one server, two clients:

\`\`\`bash
# Terminal 1
cargo run --bin spaze-server

# Terminal 2
cargo run --bin spaze -- --name andreas

# Terminal 3
cargo run --bin spaze -- --name beth
\`\`\`

Each client opens an alt-screen TUI with sidebar (server + room), tab strip, room header, message timeline, and input box. Press `i` to enter Insert mode, type a message, press Enter to send. `Esc` returns to Normal mode. `Tab` cycles tabs (room ↔ help). `Ctrl+B` toggles the sidebar. `?` opens the help buffer. `Ctrl+C` quits cleanly. `q` also quits.

Authors render as their display names (the `--name` value) in `<andreas>` style. Catppuccin Mocha is the theme.
```

(In the actual file, use real backticks not escaped ones.)

- [ ] **Step 6: Verify CI still passes locally**

```bash
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Expected: 32 tests pass, all checks clean.

- [ ] **Step 7: Commit**

```bash
git add DESIGN.md README.md
git commit -m "docs: mark Phase 1.B complete, collapse hierarchy to 2 levels" -m "Server == Space simplification documented in DESIGN.md. README updated with Phase 1.B status and Running instructions for the TUI demo."
```

---

## Task 14: Push branch + verify CI

**Files:** none.

- [ ] **Step 1: Verify all commits**

```bash
git log --oneline main..HEAD
```

Expected: ~13 commits (one per Task 1–13).

- [ ] **Step 2: Push the branch**

```bash
git push -u origin phase-1b
```

Expected: push completes, prints branch tracking info.

- [ ] **Step 3: Note CI behavior**

The workflow file restricts the `push:` trigger to `main` only. Pushing to `phase-1b` does NOT fire CI directly. CI runs when the PR is opened (Task 15).

---

## Task 15: Open PR + merge (AUTHORIZATION GATE)

**Files:** none.

Per Andreas's CLAUDE.md, PR opening and merge are user actions. The plan presents the exact commands.

- [ ] **Step 1: Open the PR**

```bash
gh pr create --base main --head phase-1b \
  --title "Phase 1.B: TUI shell + buffer abstraction + theming scaffold" \
  --body "$(cat <<'EOF'
## Summary

Second sub-project of Phase 1. Replaces stdin/stdout client with a real ratatui+crossterm TUI. Foundational `Buffer` enum (Room + Help variants in 1.B; Pins/Files/Notez/Todos/Users/Settings come in Phase 5+). 7-region layout: topbar / sidebar (toggleable) / tab strip (auto-wrap) / room header / buffer / input / status bar. Theme module with Catppuccin Mocha as the one built-in. Two terminals chat with author *names* (not hex prefixes anymore).

Locked design decisions from brainstorming:
- **Server == Space.** UI flattens to two levels; protocol's `SpaceId` retained but always 1-per-server.
- **DMs per-server**, sidebar @-prefix subgroup, separate tab color from rooms.
- **`author_display_name`** added to `PostMessage` (proto bump 0.2.0 → 0.3.0).
- **Buffer enum dispatch** (not `dyn Trait`) — compile-time exhaustiveness, no dynamic dispatch overhead.
- **Modal input** (Normal/Insert), `Ctrl+B` sidebar toggle, **60×20 floor** (50×20 with sidebar hidden).

Spec: `docs/superpowers/specs/2026-05-05-phase-1b-tui-shell-design.md`
Plan: `docs/superpowers/plans/2026-05-05-phase-1b-tui-shell.md`

## Test plan

- [x] `cargo test --workspace --all-targets` — 32 green (16 proto + 4 server + 12 client)
- [x] `cargo clippy --workspace --all-targets -- -D warnings` — clean
- [x] `cargo fmt --all -- --check` — clean
- [x] Manual: 3-terminal demo per README `## Running` — chat, mention, tab cycle, sidebar toggle, resize, Ctrl+C all work
- [x] Author names render as `<andreas>` not hex prefixes
- [x] Help buffer reachable via `?`, scrollable with j/k
- [x] Terminal mode restored on every exit path (no garbled state)

## Wire protocol changes

- `ClientCommand::PostMessage` and `Message` gain `author_display_name: String`. Workspace version 0.2.0 → 0.3.0. Server passes the field through; trusted-from-client until Phase 3 auth validates.

## Out of scope (next sub-projects)

- 1.C — Slash command parser (`/quit`, `/help`, `/connect`, etc.); also unlocks sidebar room-switching (arrow keys + Enter to "open" a room as a tab).
- Phase 2 — SQLite persistence, real multi-room/multi-server, presence events.
- Phase 3 — GitHub OAuth, server-validated identity.
- Phase 4 — Mouse, full theme system (8 built-ins + user-loadable), syntect for code blocks, pulldown-cmark for markdown.
EOF
)"
```

Open the PR URL in a browser. Verify the four CI jobs (`check`, `test`, `clippy`, `fmt`) start and pass within ~30 seconds each.

- [ ] **Step 2: Merge**

After CI is green, merge with `--merge` (preserves the per-task commit history, same as Phase 1.A):

```bash
gh pr merge --merge
```

- [ ] **Step 3: Cleanup**

```bash
git checkout main
git pull origin main
git branch -d phase-1b
gh api -X DELETE repos/Gaurgle/spaze/git/refs/heads/phase-1b
```

---

## Done

After Task 15, Phase 1.B is shipped on `main`. Status:
- `spaze-client` is now a real TUI client with foundational buffer abstraction.
- Catppuccin Mocha is the default and only theme; Phase 4 ships seven more.
- Two terminals can chat with author names rendered, mention highlighting, scrollback.
- Help buffer demonstrates the abstraction works for >1 buffer kind.

**Next:** brainstorm and plan **Phase 1.C** — slash command parser. That unlocks `/join`, `/leave`, `/quit`, `/help`, `/connect`, `/theme`, plus sidebar room-switching when users select rooms via the parser. Phase 1.C is small (~3-5 evenings), matches 1.A's complexity, and finishes the Phase 1 sub-project trilogy.
