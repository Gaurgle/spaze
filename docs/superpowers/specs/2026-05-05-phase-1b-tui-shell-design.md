# Spaze Phase 1.B — TUI shell + buffer abstraction + theming scaffold — Design

> **Status:** design locked, ready for implementation plan.
> **Sub-project of:** Phase 1 (week 1 of the 8-week MVP).
> **Sibling specs:** 1.A (`2026-05-04-phase-1a-ws-mvp-design.md`, shipped). 1.C (slash command parser, not yet planned).

## Context

Phase 1.A shipped a working bare-WebSocket chat — two terminals exchange messages through a single server via stdin/stdout. Phase 1.B replaces stdin/stdout with a real TUI built on `ratatui` + `crossterm`, introduces the foundational **buffer abstraction** that the entire client UI is built on, and lays in the **theming module scaffold** with one built-in theme so future buffers and views never reach for hardcoded colors.

The buffer abstraction is the architectural keystone of Spaze's client. Every "view" — chat rooms, DMs, pinned messages, files, notez, todos, users, settings, help — is a buffer. This spec validates the abstraction by implementing two buffer kinds (`RoomTimelineBuffer` and `HelpBuffer`) and laying out the rest of the layout structure (sidebar with servers/rooms/DMs, tab strip, room header, input box, status bars) so subsequent phases plug new buffer kinds in mechanically.

## Goal of 1.B

Same two-terminals-chat deliverable as 1.A, but with a real TUI on each client. Plus: a help buffer is reachable via `?`, the buffer abstraction is real (two kinds use it, dispatch works), the theme module exists with one built-in (Catppuccin Mocha), and the scaffolded layout (sidebar, tabs, room header, status bars) renders correctly even though most of its slots are placeholders for later phases.

The implicit deliverable: by end of 1.B, the demo screenshot looks like *working chat* — author names rendered (not hex prefixes), Catppuccin Mocha colors, sidebar showing servers/rooms/DMs, tab strip showing the room and help tab.

## Scope

### In scope

- `ratatui` + `crossterm` integration in `spaze-client`.
- `Buffer` enum with two implemented variants: `Room(RoomTimelineBuffer)` and `Help(HelpBuffer)`. Other variants (Pins, Files, Notez, Todos, Users, Settings, DirectMessage) declared in the enum or stubbed for later phases — exact list deferred to implementation.
- 7-region layout: topbar / sidebar (toggleable) / tab strip / room header (conditional) / buffer / input / statusbar.
- Sidebar shows servers (one hardcoded), rooms (one hardcoded), DMs (placeholder), repo bindings (placeholder strings since Phase 7 owns real repo linking).
- Tab strip with auto-wrap when tabs exceed width (cap at 3 rows).
- `Theme` struct with semantic slots; one built-in (`CATPPUCCIN_MOCHA`) hardcoded.
- `App` state machine: open buffers, active buffer index, sidebar visibility, input mode, input buffer, theme, connection state, identity.
- Modal input: Normal mode and Insert mode. `Esc` returns Normal; `i` enters Insert.
- Keybindings: `Tab` / `Shift-Tab` cycle, `Ctrl+B` toggle sidebar, `Ctrl+C` / `q` quit, `i` enter input, `?` open Help, `Esc` return Normal, `Enter` send message in Insert. Buffer-local: `PageUp/Down/Home/End` (Room), `j/k/Up/Down` (Help).
- Minimum terminal size enforcement (60×20). Below: render a single "terminal too small" message.
- Wire protocol change: `ClientCommand::PostMessage` gains `author_display_name: String`. Workspace version bumps 0.2.0 → 0.3.0.
- DESIGN.md update: collapse the three-level hierarchy table to two levels (Server = Space, with rooms inside).
- Unit tests: 8 (theme, app init, buffer state, scroll, tab cycle, mode transitions, mention detection).
- Integration tests: 2 lifecycle tests (`run_returns_on_ctrl_c`, `run_returns_on_server_close`) extending `tests/two_client_chat.rs`.

### Out of scope (deferred)

| Feature | Lands in |
|---|---|
| Slash commands | Phase 1.C |
| Sidebar navigation (arrow keys to walk servers/rooms, Enter to open as tab) | Phase 1.C |
| Mouse handling (region-based dispatch) | Phase 4 |
| Theme switching at runtime / 7 more built-ins / user-loadable themes | Phase 4 |
| Real repo linking | Phase 7 |
| Pins / Files / Notez / Todos / Users / Settings buffers (real implementations) | Phase 5/6 |
| Multi-server connection (sidebar shows multiple, all connected) | Phase 2 |
| Multi-room (real `JoinRoom` flow, room registry) | Phase 2 |
| Persistence / scrollback beyond session | Phase 2 |
| Auth (GitHub OAuth Device Flow) | Phase 3 |
| Reconnection logic | Phase 2 |
| Buffer scroll state for `HelpBuffer` beyond simple line offset | post-MVP |
| Tab strip horizontal scroll when 3-row wrap is exceeded | when actually needed |
| `UserJoined` / `UserLeft` / `Typing` event rendering in the room buffer | Phase 2 |
| Edit / Delete UX in the room buffer | Phase 2 |

## Locked Decisions

These were settled during the 2026-05-05 brainstorming session.

1. **Server == Space.** UI flattens to two levels: server (URL) → rooms. Multi-Space-per-server is dropped from the UI; protocol's `SpaceId` is retained but always 1-per-server. If someone wants two Spaces, they run two `spaze-server` instances (Mastodon-style). DESIGN.md's Phased Scope row 1 and the three-level-hierarchy table get updated as part of 1.B.

2. **DM placement.** DMs are Rooms with `kind = direct`, scoped per-server (Slack-like, not Discord-global). In the sidebar, DMs appear as a `DIRECT MESSAGES` subgroup beneath each server's `ROOMS` subgroup, prefixed with `@`. In the tab strip, DM tabs use the same strip as room tabs but a different color tint (yellow vs cyan).

3. **Author display names on the wire.** `ClientCommand::PostMessage` gains `author_display_name: String`. Client sends its `--name` value with each post. Server includes it in the broadcasted Message. Render uses the display name; falls back to the 8-char hex prefix if the field is empty (defensive — shouldn't happen since the field is required, but the fallback makes the renderer robust). Phase 3 replaces the source with a server-validated GitHub login; the wire field stays.

4. **Buffer abstraction shape: enum dispatch.** `Buffer` is an `enum` with one variant per kind, not `Box<dyn Trait>`. Trade-off: enum has compile-time exhaustiveness (forces every method to handle every variant, catches forgotten-arm bugs), no dynamic dispatch overhead. Adding a kind requires touching the enum, which is good — it surfaces to every method that might need to handle it. ~9 kinds at MVP; manageable.

5. **`Buffer` enum methods.** Four: `title(&self) -> &str`, `kind(&self) -> BufferKind`, `render(&mut self, frame, area, theme)`, `handle_key(&mut self, key) -> bool`. Data feed-in (e.g., `RoomTimelineBuffer::push_message`) is per-variant, not on the trait — App matches on `Buffer::Room(b)` and calls the concrete method.

6. **App owns buffers as `Vec<Buffer>`.** `App::buffers` is the open-tabs list in display order; `App::active: usize` indexes into it. Tab rendering iterates `buffers`; main panel renders only `buffers[active]`.

7. **Layout is the 7-region split** — topbar / sidebar (toggleable) / tab strip (auto-wrap, max 3 rows) / room header (conditional, only for `Room` and `DirectMessage` kinds) / buffer (flex) / input (1–3 rows) / statusbar.

8. **Minimum terminal size 60×20 with sidebar visible.** Below that: single-message error screen ("terminal too small"). When sidebar is hidden via `Ctrl+B`, layout proceeds at narrower widths.

9. **Theme has 19 semantic slots**, all bound to colors per built-in. Components reference slots, never literal colors. Catppuccin Mocha is the one built-in shipped in 1.B; 7 others + user-loadable arrive in Phase 4.

10. **Modal input.** Default: Normal mode. `i` enters Insert (input box has focus, keystrokes append to `app.input_buffer`). `Esc` returns to Normal (input buffer preserved). Other keys are mode-aware.

## Required wire protocol change

`ClientCommand::PostMessage` gains one field:

```rust
PostMessage {
    room_id: RoomId,
    author_id: UserId,
    author_device_id: DeviceId,
    author_display_name: String,    // NEW (1.B)
    body: MessageBody,
},
```

Workspace version bumps **0.2.0 → 0.3.0**. Two existing proto tests need updating (`client_command_roundtrips_json`, `post_message_carries_author_fields`). Optionally one new test that asserts `author_display_name` round-trips.

In Phase 3 the server validates `author_id` against the authenticated GitHub session and *populates* `author_display_name` from the GitHub profile (server-authoritative). Until then it's trusted-from-client.

The server side gains the field passively — `Message.body` already has the field via existing struct shape; we add the field to the `PostMessage` command and to `Message` itself if it's not there yet. (Actually, per 1.A's design `Message` does not currently carry display name — only `author_id` / `author_device_id`. So `Message` also gets `author_display_name: String`. Same proto bump.)

## Crate layout

| Crate | Change |
|---|---|
| `spaze-proto` | Add `author_display_name: String` to `PostMessage` and to `Message`. Update existing tests. Bump workspace version 0.2.0 → 0.3.0. |
| `spaze-server` | One small change: `handle_command`'s `PostMessage` arm passes the new field from command to canonical `Message` construction. No other server changes — the broadcast path is identical. |
| `spaze-client` | Major change. Adds `ratatui`, `crossterm` deps. New modules: `app.rs`, `tui.rs`, `theme/mod.rs`, `theme/catppuccin_mocha.rs`, `buffers/mod.rs`, `buffers/room_timeline.rs`, `buffers/help.rs`. Existing `render.rs` is deleted (its logic absorbed into `room_timeline.rs`). `lib.rs` `run()` loop replaces stdin reading with crossterm event stream. `main.rs` unchanged (still a thin clap wrapper). |
| `spaze-crypto` | Untouched (Phase 3). |
| `spaze-storage` | Untouched (Phase 2). |
| `spaze-commands` | Untouched (Phase 1.C). |

### New workspace dependencies

```toml
ratatui = "0.29"
crossterm = "0.28"
```

`crossterm` is the terminal backend ratatui defaults to.

### File-level layout

```
spaze-client/src/
├── main.rs           # unchanged from 1.A
├── lib.rs            # run() loop now uses crossterm event stream
├── identity.rs       # unchanged
├── app.rs            # NEW: App state machine
├── tui.rs            # NEW: layout primitives (draw fn, region split, setup/teardown)
├── theme/
│   ├── mod.rs        # NEW: Theme struct (19 slots)
│   └── catppuccin_mocha.rs  # NEW: the one built-in
└── buffers/
    ├── mod.rs        # NEW: Buffer enum, BufferKind, dispatch helpers
    ├── room_timeline.rs  # NEW: RoomTimelineBuffer (replaces render.rs)
    └── help.rs       # NEW: HelpBuffer
```

## Layout primitives

Seven named regions, top-to-bottom and left-to-right:

| Region | Height | Width | Content |
|---|---|---|---|
| `topbar` | 1 row | full | identity + 8-char self-prefix + connection state + server URL + active server's repo origin |
| `sidebar` | flexes (everything except topbar/statusbar) | fixed ~24 cols | servers (collapsible), rooms with optional repo binding, DMs subgroup, "+ Add server" footer, icon strip (search / settings / help) |
| `tab_strip` | 1–3 rows auto-wrap | full minus sidebar | open buffer tabs in `App::buffers` order |
| `room_header` | 1 row (conditional) | full minus sidebar | name, repo binding, topic — only for `Room` and `DirectMessage` kinds |
| `buffer` | flex (largest) | full minus sidebar | active buffer's `render()` — renders into this Rect |
| `input` | 1–3 rows | full minus sidebar | input box; single-line in 1.B (multi-line cap at 3 deferred to polish) |
| `statusbar` | 1 row | full | mode (NORMAL/INSERT) + active buffer name + message count + connection summary + theme name |

Sidebar toggle: `Ctrl+B`. When hidden, the layout reflows so tab_strip / room_header / buffer / input span full width. Topbar and statusbar always full-width.

Minimum size: 60 cols × 20 rows with sidebar visible. Below: render `"terminal too small (need 60×20)"` centered. When sidebar is hidden, layout proceeds at narrower widths.

`tui.rs::draw(frame, &mut app)` is called once per render cycle. It slices the screen with ratatui's `Layout` API, dispatches each region to a small helper (`draw_topbar`, `draw_sidebar`, `draw_tab_strip`, `draw_room_header`, `draw_input`, `draw_statusbar`), and calls `app.buffers[app.active].render(...)` for the buffer region. Each helper takes a `Rect` and `&Theme` so colors are theme-driven.

`tui.rs::setup()` enables raw mode + alt screen via crossterm. `tui.rs::teardown()` reverses it. RAII via a guard struct ensures terminal restoration even on panic.

## Buffer trait shape

`Buffer` is an `enum`, not a trait. Methods are pub fns on the enum that match each variant.

```rust
// spaze-client/src/buffers/mod.rs

pub enum Buffer {
    Room(RoomTimelineBuffer),    // also used for DMs (kind=Direct on inner struct)
    Help(HelpBuffer),
    // Phase 5+: Pins(PinsBuffer), Files(FilesBuffer), Notez(NotezBuffer),
    //          Todos(TodosBuffer), Users(UsersBuffer), Settings(SettingsBuffer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferKind {
    Room, DirectMessage,
    Pins, Files, Notez, Todos, Users,
    Help, Settings,
}

impl Buffer {
    pub fn title(&self) -> &str;
    pub fn kind(&self) -> BufferKind;
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
    pub fn handle_key(&mut self, key: KeyEvent) -> bool;
}
```

Each method is a `match self { Buffer::Room(b) => b.method(...), Buffer::Help(b) => b.method(...), ... }`. The compiler enforces exhaustiveness — adding a variant requires updating every method.

DM rendering note: a `RoomTimelineBuffer` whose `kind` field is `RoomKind::Direct` (proto-level kind, distinct from `BufferKind`) renders differently — `@` prefix on the title, no repo binding line, partner status in place of topic. Same struct, different render branch. The enum could split into `Room` and `DirectMessage` variants if the divergence grows; for 1.B one variant suffices.

## Theme module

`Theme` struct with 19 semantic slots:

```rust
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

    // syntect (Phase 4)
    pub syntect_theme_name: &'static str,
}
```

One built-in: `CATPPUCCIN_MOCHA` in `theme/catppuccin_mocha.rs`. Hex values from Catppuccin's official palette. All 19 slots filled; `unit test` in `lib.rs` verifies no slot is left default.

## App state + main loop

### `App` struct

```rust
pub struct App {
    pub buffers: Vec<Buffer>,         // open tabs, render order
    pub active: usize,                // index into buffers
    pub sidebar_visible: bool,        // Ctrl+B toggle
    pub mode: InputMode,              // Normal | Insert
    pub input_buffer: String,         // in-progress text
    pub theme: Theme,                 // active theme (always Catppuccin Mocha in 1.B)
    pub connection: ConnectionState,
    pub identity: Identity,
    pub should_quit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode { Normal, Insert }

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Connecting,
    Connected { server_url: String },
    Disconnected { reason: String },
}

pub struct Identity {
    pub user_id: UserId,
    pub device_id: DeviceId,
    pub display_name: String,
}
```

Constructed once at startup in `App::new(&config)`. Initial state:
- `buffers = vec![Buffer::Room(initial_room), Buffer::Help(HelpBuffer::new())]`
- `active = 0` (room is initial focus)
- `sidebar_visible = true`
- `mode = InputMode::Normal`
- `input_buffer = String::new()`
- `theme = CATPPUCCIN_MOCHA`
- `connection = ConnectionState::Connecting`
- `identity` from `config`
- `should_quit = false`

### Main loop (`spaze-client/src/lib.rs::run`)

Same `tokio::select!` shape as 1.A. Replaces stdin handling with `crossterm::event::EventStream`.

```rust
pub async fn run(config: ClientConfig) -> Result<()> {
    let (ws, _) = tokio_tungstenite::connect_async(&config.server_url).await
        .with_context(|| format!("failed to connect to {}", config.server_url))?;
    let (mut sink, mut stream) = ws.split();

    let mut app = App::new(&config);
    app.connection = ConnectionState::Connected { server_url: config.server_url.clone() };

    let mut terminal = tui::setup()?;
    let mut events = crossterm::event::EventStream::new();

    terminal.draw(|f| tui::draw(f, &mut app))?;

    let result: Result<()> = async {
        while !app.should_quit {
            tokio::select! {
                event = events.next() => {
                    handle_terminal_event(event, &mut app, &mut sink, &config).await?;
                }
                incoming = stream.next() => {
                    handle_ws_event(incoming, &mut app)?;
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
    result
}
```

`handle_terminal_event` matches on `Event::Key(k)` and dispatches per Section 5's keybinding table. `Event::Resize(_, _)` is a no-op (the redraw at end-of-loop picks up the new size). `Event::Mouse(_)` is ignored in 1.B.

`handle_ws_event` matches incoming `ServerFrame`s:
- `Response { result: Ok(MessagePosted(m)) }` → push to the room buffer matching `m.room_id` (in 1.B, the only room).
- `Event(MessagePosted(m))` → push to the matching room buffer.
- `Response { result: Err(error) }` → set `connection` (or render an error toast — small detail, 1.B can be minimal).
- Other variants → no-op (Phase 2's events).

Disconnect handling (server close / WS error) sets `connection = Disconnected { reason }`. The render shows the disconnect state in the topbar / statusbar. `app.should_quit` is set so the loop exits gracefully. Terminal is restored via `tui::teardown` regardless of how the loop exited.

## The two buffers

### `RoomTimelineBuffer`

```rust
pub struct RoomTimelineBuffer {
    pub room_id: RoomId,
    pub display_name: String,            // "# proto" / "@ beth"
    pub kind: RoomKind,                  // Standard | Direct (proto-level)
    pub repo_binding: Option<RepoBinding>,  // None in 1.B (Phase 7 wires real bindings)
    pub topic: Option<String>,            // None in 1.B (Phase 2 adds room metadata)
    pub messages: Vec<Message>,
    pub scroll: ScrollState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomKind { Standard, Direct }

pub struct RepoBinding {
    pub repo_name: String,                // "spaze-proto"
    pub subpath: Option<String>,          // Some("server/") for monorepo subdirs
}

pub struct ScrollState {
    pub offset_from_bottom: u16,
    pub stuck_to_bottom: bool,            // true when offset == 0 and user hasn't scrolled
}

impl RoomTimelineBuffer {
    pub fn push_message(&mut self, msg: Message);
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
    pub fn handle_key(&mut self, key: KeyEvent) -> bool;
}
```

Render: messages bottom-up. Each message line is `<ts> <author_display_name> body`. Timestamp uses `theme.muted`. Author uses `theme.primary`. Body uses `theme.foreground`. Mention detection: if body contains `@<self.display_name>` (case-insensitive), apply `theme.mention` background to the line. Empty body for deleted messages renders `<deleted>` in `theme.subtle`.

Scroll: bottom-stuck by default. `PageUp` / `Home` scrolls back; `PageDown` / `End` returns to bottom and re-sticks. New messages auto-scroll to bottom only if `stuck_to_bottom == true`.

`handle_key` consumes scroll keys (`PageUp/Down/Home/End`); returns `false` for everything else (bubbles to App-level handling).

### `HelpBuffer`

```rust
pub struct HelpBuffer {
    pub scroll: u16,
}

impl HelpBuffer {
    pub fn new() -> Self;
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
    pub fn handle_key(&mut self, key: KeyEvent) -> bool;
}
```

Content is a `&'static [(&str, &str)]` of `(key, description)` tuples baked into the binary. Renders as a two-column list with `theme.primary` for keys and `theme.muted` for descriptions, with a header line "Spaze · Phase 1.B keybindings" in `theme.foreground`.

`handle_key` consumes `Up/Down/j/k`; saturates at 0 on top, doesn't bound on bottom (ratatui's overflow handling clamps).

## Keybindings

### Global keys (Normal mode)

| Key | Action |
|---|---|
| `Tab` | Cycle to next tab |
| `Shift-Tab` | Cycle to previous tab |
| `Ctrl+B` | Toggle sidebar |
| `Ctrl+C` | Quit |
| `q` | Quit |
| `i` | Enter Insert mode |
| `?` | Open Help buffer (creates if not present, focuses it) |

### Buffer-local (Normal mode)

| Key | Buffer | Action |
|---|---|---|
| `PageUp` / `PageDown` / `Home` / `End` | Room | Scroll timeline |
| `j` / `k` / `Up` / `Down` | Help | Scroll help text |

### Insert mode (input box has focus)

| Key | Action |
|---|---|
| Any printable | Append to `app.input_buffer` |
| `Backspace` | Delete one char |
| `Enter` | Build `ClientFrame { command: PostMessage { ... } }`, send to sink, clear input. Stay in Insert mode. |
| `Esc` | Return to Normal mode (input buffer preserved) |

Sidebar navigation (arrow keys to walk the tree, Enter to "open" a room as a tab) is deferred to **Phase 1.C**. In 1.B the sidebar is visible-but-inert.

## Testing

### Unit tests (in `spaze-client/src/lib.rs`'s `tests` module)

| Test | What it verifies |
|---|---|
| `theme_catppuccin_mocha_has_all_slots` | Every `Theme` field is non-default (smoke test against a forgotten slot when adding builtins). |
| `app_initializes_with_two_buffers` | `App::new(&config)` produces 2 buffers; `active == 0`. |
| `room_buffer_push_message_appends` | `push_message` extends `messages`. |
| `room_buffer_scroll_sticks_to_bottom_on_new_message` | New message keeps user at bottom if they were stuck; doesn't move viewport if user scrolled up. |
| `tab_cycle_wraps_around` | `Tab` from last → first; `Shift-Tab` from first → last. |
| `help_buffer_scrolls_with_jk_and_arrows` | Up / k decrements (saturating); Down / j increments. |
| `app_input_mode_transitions` | `i` Normal → Insert; `Esc` Insert → Normal; `input_buffer` preserved across mode swap. |
| `mention_detection_uses_self_display_name` | Body containing `@<self.display_name>` flags the message as mention. |

These are pure-state tests. No terminal I/O, no network — fast.

### Integration tests (extend `spaze-server/tests/two_client_chat.rs`)

| Test | What it does |
|---|---|
| `client_lib_run_returns_on_ctrl_c` | Spawn `spaze_client::run` in a tokio task with a fake terminal (no actual draw — `tui::setup` returns a backend with no I/O). Send a Ctrl+C signal-equivalent. Assert `run()` returns `Ok(())` without panicking. |
| `client_lib_run_returns_on_server_close` | Same harness; close the server. Assert `run()` returns with the disconnect-handling having occurred (terminal restored, no garbled output). |

These verify lifecycle, not rendering. Terminal automation is intentionally not in scope.

### Manual smoke test (the deliverable)

1. Three terminals — server, client A, client B. Server: `cargo run --bin spaze-server`. Clients: `cargo run --bin spaze -- --name andreas` and `cargo run --bin spaze -- --name beth`.
2. Both clients render the layout: sidebar with `spaze-dev` server expanded, `# proto` room visible, tab strip with `# proto` and `?` help tab, room header with name, message area empty, input box at bottom, status bar showing `NORMAL`.
3. In client A: `i` → input mode → type "hello world" → `Enter`. Both clients see `<andreas> hello world` (display name, not hex).
4. In client B: `i`, type "hi back!", Enter. Both clients see it. Author is `<beth>`.
5. Press `?` in either client. Help buffer becomes active. Tab strip shows `?` highlighted. Help text lists the keybindings.
6. Press `Tab` in either client. Cycles back to room. Press `Tab` again, back to help.
7. Press `Ctrl+B`. Sidebar hides. Layout reflows. Press again, sidebar returns.
8. Resize terminal below 60×20. "Terminal too small (need 60×20)" message centered. Resize back; layout restores cleanly.
9. `Ctrl+C` on a client. Terminal mode restored (no garbled state). Exit code 0. Server keeps running.
10. `Ctrl+C` on server. Client shows disconnected state in topbar/statusbar; eventually exits (graceful) with terminal restored.

If 1–10 all work, 1.B is done.

## Acceptance criteria

1. `cargo test --workspace --all-targets` passes (existing 22 + new 8 unit + 2 integration = 32 tests, all green).
2. `cargo clippy --workspace --all-targets -- -D warnings` clean.
3. `cargo fmt --all -- --check` clean.
4. Manual smoke test (steps 1–10 above) all pass.
5. CI on GitHub passes (`check`, `test`, `clippy`, `fmt` all green).
6. Terminal mode is restored on every exit path (Ctrl+C, server close, panic) — no garbled terminal after the binary exits.
7. Sidebar toggle (Ctrl+B) works; layout reflows correctly with sidebar hidden.
8. Tab strip auto-wraps when tab count exceeds one row at the current width.
9. The Help buffer renders correctly and lists all 1.B keybindings.

## Implementation order (for the plan)

1. **Branch + workspace deps + version bump.** Add `ratatui`, `crossterm` to `[workspace.dependencies]`. Bump 0.2.0 → 0.3.0.
2. **Proto change.** Add `author_display_name` to `PostMessage` and to `Message`. Update tests.
3. **Server change.** `handle_command`'s `PostMessage` arm passes `author_display_name` from command into the canonical `Message`.
4. **Theme module.** `Theme` struct + `CATPPUCCIN_MOCHA` constant + theme test.
5. **Buffer skeleton.** `Buffer` enum with two variants, `BufferKind`, four methods. `HelpBuffer` (small, complete). `RoomTimelineBuffer` skeleton (state struct + `push_message`; render/handle_key are stubs).
6. **App struct.** Initial state, `App::new`, `should_quit` flag.
7. **TUI setup/teardown + draw skeleton.** `tui::setup()`, `tui::teardown()`, `tui::draw()` rendering empty regions for now (just structure visible).
8. **Region rendering helpers.** `draw_topbar`, `draw_sidebar`, `draw_tab_strip`, `draw_room_header`, `draw_input`, `draw_statusbar`. Each takes `Rect` + `&Theme`.
9. **`RoomTimelineBuffer::render`.** Real message rendering with theme slots, scroll, mention detection.
10. **`HelpBuffer::render`.** Static keybindings list.
11. **Main loop integration.** Replace stdin loop with crossterm event stream + `tokio::select!`. Wire keybindings (global + Insert). Wire WS event → buffer push.
12. **Minimum size enforcement.** `MIN_COLS` / `MIN_ROWS` constants + `draw_too_small` fallback.
13. **Sidebar toggle (Ctrl+B).** State flag + layout reflow.
14. **Unit tests.** All 8.
15. **Integration tests.** 2 lifecycle tests in `tests/two_client_chat.rs`.
16. **DESIGN.md update.** Collapse the 3-level hierarchy table to 2 levels. Update Phased Scope row 1 to mark 1.B's sub-project status.
17. **README update.** "Phase 1.B complete — TUI shell + buffer abstraction + Catppuccin Mocha. TUI works in two terminals."
18. **Manual smoke test pass.** All 10 steps green.
19. **Final CI check, push branch, open PR, merge.**

Each step is committable independently. Phase 1.B is roughly 2× the size of 1.A in lines of code (TUI code is verbose), so the plan will have ~12–15 plan tasks compared to 1.A's 12.

## Open implementation questions

These don't affect the design but will need micro-decisions during implementation:

- **Exact tab-strip wrap logic.** ratatui's `Layout` doesn't auto-wrap; manual computation needed. Implementer decides the algorithm (greedy left-to-right with cumulative width).
- **`tui::setup`'s exact terminal mode setup.** Raw mode + alt screen are standard; mouse capture optional in 1.B (off by default since we don't handle mouse events).
- **RAII guard for terminal restoration.** Wrap `Terminal<Backend>` in a guard struct whose `Drop` calls `disable_raw_mode` + `LeaveAlternateScreen`. Standard ratatui pattern.
- **Panic handler.** Install a panic hook that restores the terminal before printing the panic. ratatui has `init_panic_hook` examples.
- **`tracing` output during TUI sessions.** Tracing writes to stderr (per 1.A's `main.rs`); when stderr is the alt-screen, output is invisible. Implementer to decide whether to redirect tracing to a file or accept that logs vanish during TUI. Probably a file (`~/.local/state/spaze/client.log` or similar) — defer to implementation.
- **Help buffer content list.** The keybindings table from this spec, rendered as `&'static [(&str, &str)]`.
- **Error toast UX.** When a `Response::Err` arrives, where does it surface? Status bar? Floating overlay? For 1.B: write to `app.connection` if it's a connection-level error; otherwise log via tracing. Toast UX is post-MVP polish.
