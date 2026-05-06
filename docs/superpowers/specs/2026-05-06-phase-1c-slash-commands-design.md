# Phase 1.C — Slash Command Parser

> **Status:** design locked, ready for implementation plan.
> **Sub-project of:** Phase 1 (week 1 of the 8-week MVP).
> **Sibling specs:** 1.A (`2026-05-04-phase-1a-ws-mvp-design.md`, shipped). 1.B (`2026-05-05-phase-1b-tui-shell-design.md`, shipped). 1.D (sidebar navigation, not yet planned — split out from 1.C during this brainstorm).

## Context

Phase 1.A shipped the bare WebSocket round-trip; 1.B shipped the ratatui TUI with the buffer abstraction and one built-in theme. 1.C is the third and final sub-project of Phase 1, and it stands up the slash command system: a parser, a registry with metadata, an `Effect` enum that handlers return, and four starter commands wired into the TUI's input dispatch.

The architectural keystone of 1.C is the dependency direction. `spaze-commands` becomes a self-contained crate with zero workspace dependencies — it carries the parser, registry, handlers, and `Effect` enum, all of which use only `std` types. None of the four 1.C `Effect` variants reference proto types, because effects represent *semantic intent* (e.g., `SendActionMessage(String)`), not wire-level commands; the client translates effects into wire-level `ClientCommand`s using its own state. Phase 2 may add `spaze-proto` as a dep when commands need to express richer wire-level intent, but 1.C does not need it. Commands are pure functions of `(args, context) → Vec<Effect>`. The client is the only place that translates `Effect`s into actual side effects (mutating App state, writing to the WS sink, etc.). This means the entire command surface is testable as plain unit tests with zero async setup, and the same pattern scales unchanged through Phase 2/3 as more commands land.

A second user-visible polish item is folded into 1.C: **live input coloring**. As the user types, the input bar's foreground color shifts based on whether what they're typing is a recognized command — green for a registry hit, red for a miss, normal foreground for plain text or escaped text. This costs ~30 lines (one small helper plus a renderer change) and gives instant visual feedback that doubles as an "am I in command mode?" indicator. It also replaces the most common need for system-line error reporting: typos never reach Enter.

## Goal of 1.C

Same two-terminals-chat deliverable as 1.B, plus four working slash commands (`/quit`, `/help`, `/me`, `/clear`), live input coloring that signals command-vs-text-vs-invalid as the user types, a new `MessageBody::Action` proto variant carrying `/me` content end-to-end, and the `spaze-commands` crate populated with a real registry-with-metadata pattern that future phases extend by adding entries.

The implicit deliverable: by end of 1.C, the demo screenshot includes a green `/me kicks the build` in the input bar, an `* andreas kicks the build` line in the timeline (italic, no chevrons), and the user can type `/help` to see all available commands listed inline.

## Scope

### In scope

- `spaze-commands` crate gains its real public API: parser, registry, handlers, `Effect` enum, `HandlerContext` struct.
- Four starter commands: `/quit`, `/help`, `/me`, `/clear`.
- Static `Command` registry with `{ name, aliases, short_help, long_help, handler }` metadata.
- Live input coloring in the input bar (green = valid command, red = invalid command, foreground = text or escaped text).
- `MessageBody::Action { content: String }` proto variant.
- Render arm in `RoomTimelineBuffer::render` for `Action` (italic, no `<author>` wrapper, `* {author} {content}` shape).
- Mention detection extended so `@name` inside an Action body still highlights.
- `unicode-width` dependency added to `spaze-client` for accurate cursor positioning on multi-byte / wide UTF-8 input.
- Unit + integration test coverage matching the test plan in this spec (~42 new tests; cumulative ~74 after 1.C).

### Out of scope (deferred)

| Feature | Lands in |
|---|---|
| Sidebar navigation (arrow keys to walk servers/rooms, Enter to open as tab) | Phase 1.D |
| Quoted args / multi-word args / shell-style tokenization | When a command actually needs it (likely Phase 2) |
| Plugin / runtime command registration (`Box<dyn CommandHandler>` registry) | Post-MVP plugin system |
| `/connect` (in-app server switching, multi-server) | Phase 2 |
| `/join`, `/leave`, `/topic` (room-level commands) | Phase 2 |
| `/clear` survives reconnection / scrollback (sentinel-based clear) | Phase 2 (when scrollback semantics are real) |
| Rich help formatting (categories, columns, syntax-highlighted long_help) | Phase 4+ |
| Typeahead / completion of command names while typing | Post-MVP |
| Compound-emoji width correction (ZWJ sequences, skin tones) | Terminal-dependent, never |
| Author/server-issued system messages over the wire | Phase 2 |
| Toast / status-bar error surface | If command-system surface ever proves insufficient |

## Locked Decisions

These were settled during the 2026-05-06 brainstorming session. Each is non-negotiable for 1.C; revisiting belongs in a follow-up sub-project.

1. **Scope = slash parser only.** Sidebar navigation is split out into Phase 1.D. 1.C focuses on input parsing + dispatch + four commands, nothing else.

2. **Command set = `/quit`, `/help`, `/me`, `/clear`.** Smallest set that exercises every code path: client-local effect (`/quit`, `/clear`), proto-level effect with new variant (`/me`), and registry introspection (`/help`).

3. **Effect enum architecture.** Handlers return `Vec<Effect>`. App is the only place effects become side effects. `spaze-commands` has zero workspace dependencies in 1.C; the four `Effect` variants use only `std` types. The crate never depends on `spaze-client`, `ratatui`, or `tokio`. Phase 2+ may add `spaze-proto` if a future effect needs to carry wire-level types.

4. **`MessageBody::Action { content }` proto variant.** Phase 2 will want a real action message type for search, export, and GitHub integration; doing it now (vs render-time string convention) avoids a refactor and is the smaller change. Cascade is bounded: proto + serde tests + one client render arm + `body_text` helper. Server is unchanged.

5. **Trigger condition.** Leading `/` triggers command mode. `//` escapes (sends literal text starting with `/`). Bare `/` (no name) is treated as text. Whitespace before `/` means it's text (we don't strip-then-check). Empty input (Enter on empty buffer) does nothing — guard inherited from 1.B's existing `KeyCode::Enter if !app.input_buffer.is_empty()` match arm.

   Edge case for prefix length: `///me` matches `starts_with("//")` so it's `EscapedText("//me")` — one slash stripped from any multi-slash prefix, matching the rule "to type a literal `/foo`, prefix it with one extra `/`."

6. **Parser is `split_whitespace`.** No quoting, no escapes inside args. None of the four 1.C commands need it. Phase 2 can extend the tokenizer when a command actually needs quoted args.

7. **Static `Command` registry with metadata.** Each command is a `Command { name, aliases, short_help, long_help, handler }`. Registry is `&'static [Command]`. Lookup walks the slice (N=4, no HashMap overhead). Adding a command later = one new entry. Plugin / dyn registry is explicitly post-MVP.

8. **`/help` is a distinct surface from the `?` keypress.** `?` opens the visual `HelpBuffer` (keybindings reference). `/help` (no args) prints all commands' `short_help` as a system line in the current buffer. `/help <name>` prints that command's `long_help`. They serve different audiences and don't share code.

9. **Live input coloring.** While typing in Insert mode, the input bar's foreground color reflects `classify_input` output: `theme.success` (green) for a registry hit, `theme.error` (red) for a miss, `theme.foreground` for plain text or escaped text. Lookup is command-name-only (first token after `/`); args don't influence the color.

10. **Red Enter does not submit.** If `classify_input` returned `InvalidCommand`, pressing Enter does nothing — the input stays in the bar. No system line is rendered. The visual red color is sufficient feedback. Avoids cluttering the timeline with typo errors.

11. **Green Enter that fails dispatches a system line.** When a recognized command's handler / app-application emits `Effect::SystemLine`, the line renders in the active buffer as a local-only system message (italic, dim, identical visual treatment to server-issued `MessageBody::System`). Examples: `/me` with no args, `/me` while disconnected, `/help foo` for unknown command name.

12. **`/clear` is session-local `messages.clear()` on the active buffer.** No persistence, no sentinel, no replay defense. Phase 2 will revisit when server-side scrollback is real.

## Architecture

### Crate dependency direction

```
spaze-proto (no deps on workspace crates)
    ↑
spaze-commands (zero workspace deps in 1.C)
    ↑
spaze-client (deps on spaze-proto + spaze-commands)
```

`spaze-commands` is fully self-contained: parser, registry, handlers, `Effect` enum, `HandlerContext`. Zero workspace dependencies — the crate uses only `std`. No `ratatui`, no `tokio`, no `serde`, no `spaze-proto`, no client types. Tests are pure-function tests.

### `spaze-commands` file layout

```
spaze-commands/src/
    lib.rs          (~30 lines — re-exports + crate-level doc comment)
    effect.rs       (Effect enum + small docs)
    parser.rs       (parse_input, classify_input, InputKind, InputClass)
    registry.rs     (static REGISTRY, Command struct, lookup_by_name, iter)
    commands.rs     (~150 lines — all four Command entries + their handler fns; #[cfg(test)] block)
```

Five files. Single `commands.rs` is correct for 1.C's four commands; if Phase 2/3 adds enough commands that the file exceeds ~250 lines, split into a `commands/` directory then. The reverse (collapse from per-file to single file) is also a five-minute refactor. We optimize for "smaller now."

### `spaze-client` integration points

- `app.rs` gains `fn apply_effect(&mut self, effect: Effect, ws_tx: &mpsc::Sender<ClientFrame>)`. This is the *only* place effects become side effects (mutating `should_quit`, clearing buffers, pushing system lines, sending WS frames).
- `lib.rs` Insert-mode Enter handler changes from "always send as text" to a three-way branch on `parse_input`'s output: Text / EscapedText / Command (with sub-branch on registry lookup hit/miss).
- `tui.rs` input bar renderer takes the registry, calls `classify_input`, picks the foreground color, and uses `unicode-width` for cursor positioning.
- `Cargo.toml` adds `spaze-commands.workspace = true` and `unicode-width = "0.1"` to `spaze-client`.

## Required proto change

`MessageBody` gains one new variant:

```rust
// spaze-proto/src/message.rs (or equivalent)
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MessageBody {
    Text   { content: String },
    System { content: String },
    Action { content: String },  // NEW — carries /me content
}
```

That's the entire proto change. `Message` and `PostMessage` already hold a `MessageBody`, so they need no field changes.

### Cascade

| Layer | Change | Approx. lines |
|---|---|---|
| `spaze-proto::message` | Add `Action` variant. | ~3 |
| `spaze-proto` serde tests | Add round-trip tests for Action (ASCII + UTF-8 + within full Message). | ~25 |
| `spaze-server` | **No change.** Server passes `MessageBody` through opaquely. Existing tests cover the code path. | 0 |
| `spaze-client::buffers::room_timeline` render | Add `Action` match arm: italic, `* {author} {content}` shape. | ~10 |
| `spaze-client::buffers::room_timeline::body_text` helper | Add `Action` to the matched-cases list so mention detection works inside actions. | ~1 |
| `spaze-client` integration tests | New: `/me` round-trip between two clients carries `Action` body intact. | ~30 |

### Render specification for Action

Standard message render (already shipped in 1.B):

```
12:34 <andreas> hello world
```

Action render (new):

```
12:34  * andreas kicks the build
```

Differences:
- No chevrons around author name.
- Body text is italic.
- Two-space gap after timestamp instead of one (subtle visual offset).
- No `<>` wrapper anywhere on the line.
- Mention highlight rules apply unchanged: if body contains `@{self_display_name}` (case-insensitive), the entire line gets the `theme.mention` background with dark text.

The `*` prefix is the universal IRC convention for actions; users coming from any IRC client / Slack `/me` recognize it instantly.

## Data flow

### Keystroke flow (Insert mode, every char)

```
KeyPress(c)
    ↓
app.input_buffer.push(c)
    ↓
next render frame
    ↓
classify_input(&buf, REGISTRY) → InputClass
    ↓
pick fg by InputClass:
    Text             → theme.foreground
    EscapedText      → theme.foreground
    ValidCommand(_)  → theme.success
    InvalidCommand(_)→ theme.error
    ↓
render input bar with chosen fg
    ↓
position cursor at unicode_width::UnicodeWidthStr::width(&buf) cells from area.x
```

`classify_input` is a pure function on `&str`. It allocates nothing — the `name` field of `ValidCommand`/`InvalidCommand` borrows from the input. Cost per keystroke: one `starts_with` + one O(N) registry scan (N=4 in 1.C; even at N=50 in Phase 4 a linear scan is faster than a HashMap due to cache effects on small slices).

### Enter flow (Insert mode, submission)

```
KeyPress(Enter)
    ↓
guard: if app.input_buffer.is_empty() → do nothing, return
    ↓
parse_input(&buf) → InputKind
    ↓
match InputKind:
    Text(content) | EscapedText(content):
        ws_tx.send(ClientFrame::Command(PostMessage{ body: Text(content), ... }))
        app.input_buffer.clear()

    Command { name, args }:
        match REGISTRY.iter().find(|c| c.name == name || c.aliases.contains(&&name[..])):
            Some(cmd):
                let ctx = HandlerContext { registry: REGISTRY };
                let effects = (cmd.handler)(&args.iter().map(String::as_str).collect::<Vec<_>>(), &ctx);
                for effect in effects:
                    app.apply_effect(effect, &ws_tx);
                app.input_buffer.clear();
            None:
                // do nothing — input stays, no submission, no system line
                // (matches the "red Enter doesn't submit" rule)
```

`EscapedText`'s content has its leading slash already stripped by `parse_input`. So `//me kicks` parses to `EscapedText("/me kicks")` and gets sent as text `/me kicks`, exactly what the user wanted to send literally.

### Effect application

```rust
fn apply_effect(&mut self, effect: Effect, ws_tx: &mpsc::Sender<ClientFrame>) {
    match effect {
        Effect::Quit => {
            self.should_quit = true;
        }
        Effect::ClearActiveBuffer => {
            if let Some(room) = self.active_room_buffer_mut() {
                room.messages.clear();
                room.scroll.to_bottom();
            }
        }
        Effect::SystemLine(content) => {
            self.active_buffer_mut().push_local_system(content);
        }
        Effect::SendActionMessage(content) => {
            if !self.connection.is_connected() {
                self.apply_effect(
                    Effect::SystemLine(
                        "not connected — /me requires an active connection".into()
                    ),
                    ws_tx,
                );
                return;
            }
            let post = self.build_post_message(MessageBody::Action { content });
            let _ = ws_tx.try_send(ClientFrame::Command(ClientCommand::PostMessage(post)));
        }
    }
}
```

Connectivity check lives in `apply_effect`, not in handlers. Handlers stay pure. Future commands that send proto messages get the disconnected-gracefully behavior for free.

`push_local_system(content)` is a small new method on `Buffer` that appends a synthetic `Message` with `body: MessageBody::System { content }`, `MessageId::nil()`, `author_id::nil()`, `author_display_name: "spaze"` (or empty), and the current timestamp. Locally generated, never sent over the wire, never persisted (1.C has no persistence anyway). Visually identical to server-issued System messages — that's the point.

## Public APIs of `spaze-commands`

### `parser` module

```rust
pub fn classify_input<'a>(input: &'a str, registry: &[Command]) -> InputClass<'a>;
pub fn parse_input(input: &str) -> InputKind;

pub enum InputClass<'a> {
    Text,
    EscapedText,
    ValidCommand   { name: &'a str },
    InvalidCommand { name: &'a str },
}

pub enum InputKind {
    Text(String),
    EscapedText(String),                          // input with the leading slash stripped
    Command { name: String, args: Vec<String> },
}
```

Two functions because the cost profile differs. `classify_input` runs every keystroke and must not allocate. `parse_input` runs at most once per Enter and needs owned strings to outlive `app.input_buffer.clear()`.

`classify_input` rules (precise):

1. If `input.starts_with("//")` → `EscapedText`.
2. Else if `input.strip_prefix('/')` succeeds, take the first whitespace-delimited token of the remainder as the candidate name:
   - If the name is empty (bare `/`, or `/   `) → `InvalidCommand { name: "" }`.
   - If any registry entry matches by `name` or `aliases.contains(name)` → `ValidCommand { name }`.
   - Otherwise → `InvalidCommand { name }`.
3. Else → `Text`.

`parse_input` rules (precise):

1. If `input.starts_with("//")` → `EscapedText(input[1..].to_string())` (one slash stripped).
2. Else if `input.strip_prefix('/')` succeeds, split the remainder by whitespace; first token is `name` (empty string if remainder was empty/whitespace), rest become `args`. Returns `Command { name, args }`.
3. Else → `Text(input.to_string())`.

### `effect` module

```rust
pub enum Effect {
    Quit,
    ClearActiveBuffer,
    SystemLine(String),
    SendActionMessage(String),  // /me <content>
}
```

Four variants in 1.C. Future variants land additively as needed.

`SendActionMessage` is at the *semantic intent* level, not the wire level. The handler doesn't construct a full `PostMessage` — that requires `request_id`, `room_id`, `author_id`, `author_display_name`, all of which are app state. `app.apply_effect` does the wire-level translation, which keeps handlers pure and gives us one place to handle disconnection / connectivity defenses.

### `registry` module

```rust
pub type Handler = fn(args: &[&str], ctx: &HandlerContext) -> Vec<Effect>;

pub struct HandlerContext<'a> {
    pub registry: &'a [Command],
}

pub struct Command {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub short_help: &'static str,
    pub long_help: &'static str,
    pub handler: Handler,
}

pub static REGISTRY: &[Command] = &[ /* the four 1.C commands, defined in commands.rs */ ];

pub fn lookup<'a>(name: &str, registry: &'a [Command]) -> Option<&'a Command>;
```

`HandlerContext` is the extension point. Most handlers ignore it; `/help` walks `ctx.registry` to produce its output. Future handlers might want other things (e.g., current room name) and the context grows additively without changing existing handler signatures.

### `commands` module — the four handlers

```rust
fn handle_quit(_args: &[&str], _ctx: &HandlerContext) -> Vec<Effect> {
    vec![Effect::Quit]
}

fn handle_clear(_args: &[&str], _ctx: &HandlerContext) -> Vec<Effect> {
    vec![Effect::ClearActiveBuffer]
}

fn handle_me(args: &[&str], _ctx: &HandlerContext) -> Vec<Effect> {
    if args.is_empty() {
        return vec![Effect::SystemLine("usage: /me <action text>".into())];
    }
    vec![Effect::SendActionMessage(args.join(" "))]
}

fn handle_help(args: &[&str], ctx: &HandlerContext) -> Vec<Effect> {
    if args.is_empty() {
        // One Effect::SystemLine per output line — each becomes its own
        // Message in the buffer, each renders as its own visual line.
        // Avoids embedding `\n` inside a single SystemLine string and
        // hoping ratatui's Paragraph splits it (it doesn't, in our render path).
        let mut effects = vec![Effect::SystemLine("available commands:".into())];
        for cmd in ctx.registry {
            effects.push(Effect::SystemLine(
                format!("  /{} — {}", cmd.name, cmd.short_help),
            ));
        }
        return effects;
    }
    let target = args[0].trim_start_matches('/');
    match lookup(target, ctx.registry) {
        Some(cmd) => vec![Effect::SystemLine(format!("/{}: {}", cmd.name, cmd.long_help))],
        None      => vec![Effect::SystemLine(format!("unknown command: /{}", target))],
    }
}
```

All four are pure functions of `(args, ctx) → effects`. Unit-testable as `assert_eq!(handle_me(&[], &ctx), vec![Effect::SystemLine("usage: /me <action text>".into())])` with zero setup.

### Registry entries

```rust
pub static REGISTRY: &[Command] = &[
    Command {
        name: "quit",
        aliases: &["q"],
        short_help: "exit the client",
        long_help: "/quit — exit the Spaze client cleanly. Closes the WS connection and restores the terminal.",
        handler: handle_quit,
    },
    Command {
        name: "help",
        aliases: &["h"],
        short_help: "list commands or show help for one",
        long_help: "/help — list all commands. /help <name> — show detailed help for a specific command.",
        handler: handle_help,
    },
    Command {
        name: "me",
        aliases: &[],
        short_help: "send an action message",
        long_help: "/me <action text> — send your message as an action (rendered as '* yourname text' in the timeline).",
        handler: handle_me,
    },
    Command {
        name: "clear",
        aliases: &["cls"],
        short_help: "clear the current buffer's timeline",
        long_help: "/clear — empty the active buffer's local timeline. Session-local; messages are not re-fetched on reconnect (no scrollback yet).",
        handler: handle_clear,
    },
];
```

## Live input coloring (renderer integration)

In `spaze-client::tui` (or wherever the input bar is rendered today), the input render becomes:

```rust
use spaze_commands::parser::{classify_input, InputClass};
use spaze_commands::registry::REGISTRY;
use unicode_width::UnicodeWidthStr;

pub fn render_input(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let class = classify_input(&app.input_buffer, REGISTRY);
    let fg = match class {
        InputClass::Text | InputClass::EscapedText      => theme.foreground,
        InputClass::ValidCommand   { .. }               => theme.success,
        InputClass::InvalidCommand { .. }               => theme.error,
    };
    let para = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(fg).bg(theme.background));
    frame.render_widget(para, area);

    if app.input_mode == InputMode::Insert {
        let cursor_col = UnicodeWidthStr::width(app.input_buffer.as_str()) as u16;
        frame.set_cursor_position((area.x + cursor_col, area.y));
    }
}
```

`unicode-width` is the same crate ratatui itself uses for layout. Adding it as a direct `spaze-client` dependency costs ~50KB and gives correct cursor positioning for multi-byte and wide characters.

## Unicode / UTF-8 contract

All string-bearing types in Spaze (proto and client) are UTF-8 by construction (Rust `String` is UTF-8 native). The wire format is JSON, which is UTF-8 native. No transcoding ever happens.

Phase 1.C explicitly tests UTF-8 round-trips for:
- Plain text messages: `"café åäö"`, `"🐧 says hi"`.
- Action messages: `/me waves at åse 🐧`.
- Mention detection inside Action: `@åse` highlights when self-display-name is `"åse"`.
- Invalid-command classification of UTF-8 names: `/qüit` → `InvalidCommand { name: "qüit" }`.
- Cursor positioning on multi-byte input: `ä😀x` → display width 4 cells (1+2+1), cursor at column 4 from `area.x`.

Compound emoji width (ZWJ sequences, skin-tone modifiers) is best-effort — `unicode-width` handles base codepoints correctly, but ZWJ joins like `👨‍👩‍👧` are terminal-rendering-dependent and we don't try to fix the terminal. Documented out-of-scope.

`split_whitespace` is Unicode-aware (splits on Unicode whitespace per Rust std), so `/me  hi   there` collapses runs of any kind of whitespace into single-space-joined args correctly.

`String::pop()` (used by Backspace) removes one char (codepoint), not one byte — so deleting the last `ö` works correctly.

`to_lowercase()` (used by mention detection) does Unicode default case folding. Edge case: Turkish dotless I is not handled per Turkish locale, but that's acceptable for a chat tool and matches Phase 1.B's existing behavior.

## Error UX

Two complementary feedback channels:

### Channel 1 — Live coloring (continuous, while typing)

Already covered above. The user sees red/green as they type, never has to submit-and-wait to know if their command name is valid.

### Channel 2 — System lines on green-Enter (only when something fails post-validation)

A `Effect::SystemLine(content)` renders in the active buffer with the same visual treatment as a server-issued `MessageBody::System` (italic, dim foreground, no `<author>` wrapper, prefix `-- system: `). Locally generated, never sent to the server, never persisted.

Cases that produce a system line in 1.C:

1. `/me` (no args) → `-- system: usage: /me <action text>`
2. `/me hi` while WS is down → `-- system: not connected — /me requires an active connection`
3. `/help <unknown>` → `-- system: unknown command: /unknown`
4. `/help` (no args) → 5 system lines, one per visual line:
   ```
   -- system: available commands:
   -- system:   /quit — exit the client
   -- system:   /help — list commands or show help for one
   -- system:   /me — send an action message
   -- system:   /clear — clear the current buffer's timeline
   ```
   Each line is its own `Effect::SystemLine` and its own synthetic Message in the buffer. The `-- system:` prefix repeats per line — slightly verbose but renders correctly with no special multi-line handling. A future polish phase can introduce a "block system message" render path if this proves ugly in practice.
5. `/help me` → 1 system line: `-- system: /me: /me <action text> — send your message as an action (rendered as '* yourname text' in the timeline).`

Cases that explicitly do *not* produce a system line:

1. `/foo` (red, registry miss) Enter → input stays, no system line, no submission. Visual red was sufficient.
2. `/quit`, `/clear` (always succeed in 1.C, never error) — no system line on success path.

## Testing strategy

Total ~42 new tests across four layers. Cumulative project test count after 1.C ships: ~74 (32 from 1.B + 42 new).

### Layer 1 — `spaze-commands` unit tests (~26 tests)

Pure-function tests, zero setup, run in milliseconds.

`parser::classify_input` (10 tests):
- `""` → Text
- `"hello"` → Text
- `"hello /world"` → Text (slash mid-line, not at start)
- `"//me kicks"` → EscapedText
- `"/quit"` → ValidCommand{"quit"}
- `"/q"` → ValidCommand{"q"} (alias hit)
- `"/foo"` → InvalidCommand{"foo"}
- `"/"` → InvalidCommand{""}
- `"/   "` → InvalidCommand{""}
- `"/me hi"` → ValidCommand{"me"} (args present, name still valid)
- `"/qüit"` → InvalidCommand{"qüit"} (UTF-8)

`parser::parse_input` (4 tests):
- `"hello"` → Text("hello")
- `"//me kicks"` → EscapedText("/me kicks")
- `"/me kicks the build"` → Command{name:"me", args:vec!["kicks","the","build"]}
- `"/quit"` → Command{name:"quit", args:vec![]}

`commands::handle_quit` (1 test): always returns `vec![Effect::Quit]`.

`commands::handle_clear` (1 test): always returns `vec![Effect::ClearActiveBuffer]`.

`commands::handle_me` (3 tests):
- empty args → SystemLine with usage text
- `["kicks", "the", "build"]` → SendActionMessage("kicks the build")
- UTF-8 args `["waves", "at", "åse", "🐧"]` → SendActionMessage("waves at åse 🐧")

`commands::handle_help` (4 tests):
- no args → 5 SystemLine effects: "available commands:" plus one per registered command (header + 4 = 5)
- `["me"]` → 1 SystemLine with /me's long_help
- `["foo"]` → 1 SystemLine with "unknown command: /foo"
- `["/me"]` → 1 SystemLine with /me's long_help (leading slash stripped from the arg)

`registry` invariants (3 tests):
- No two commands share a `name`.
- No `alias` collides with another command's `name` or another command's alias.
- Every command has non-empty `name`, `short_help`, and `long_help`.

### Layer 2 — `spaze-proto` unit tests (3 tests)

- `action_body_serde_roundtrip`: `MessageBody::Action { content: "kicks" }` → JSON → deserializes equal.
- `action_body_utf8_roundtrip`: `MessageBody::Action { content: "kicks the åäö 🐧" }` round-trips.
- `message_with_action_body`: Full `Message` carrying Action body serializes correctly with all metadata.

### Layer 3 — `spaze-client` unit tests (~9 tests)

Non-async, App-level, deterministic.

- `apply_effect_quit_sets_flag`: after `apply_effect(Effect::Quit, ...)`, `app.should_quit == true`.
- `apply_effect_clear_empties_buffer`: after `apply_effect(Effect::ClearActiveBuffer, ...)`, active room buffer's `messages` vec is empty.
- `apply_effect_systemline_appends_local_message`: active buffer gains one Message with `body: System { content: <input> }` and current timestamp.
- `apply_effect_send_action_when_connected`: ws_tx mock receives `ClientFrame::Command(PostMessage{ body: Action { content: "kicks" }, ... })` with proper room_id and author fields.
- `apply_effect_send_action_when_disconnected`: ws_tx mock untouched; instead a SystemLine is pushed to the active buffer.
- `input_color_picker`: each `InputClass` variant maps to the expected theme slot.
- `cursor_width_with_emoji`: `UnicodeWidthStr::width("ä😀x") == 4`.
- `room_render_action_body`: rendering a Message with Action body produces a line shaped `* {author} {content}` (italic, no chevrons). Snapshot or assertion-based.
- `mention_in_action_body_highlights`: `/me waves at @beth` triggers the mention highlight when self_display_name = "beth".

### Layer 4 — `spaze-client` integration tests (4 tests)

Real WS code paths in-process (same pattern as 1.A's existing tests).

- `me_command_roundtrip_two_clients`: A connects, B connects, A submits `/me waves`, B receives a Message with `body == Action { content: "waves" }`.
- `clear_only_affects_local_buffer`: A and B exchange three messages, A runs `/clear`, B's buffer is unaffected (all three messages still present).
- `quit_command_sends_close_frame`: A runs `/quit`, server logs disconnect, no orphan connection.
- `me_with_utf8_roundtrip`: A submits `/me waves at åse 🐧`, B receives Action body with content `"waves at åse 🐧"` byte-equal.

## Manual smoke test (1.C merge gate)

Three-terminal smoke (server + two clients). Run after all unit and integration tests pass on the branch. Script:

1. Both clients connect with `--name andreas` and `--name beth`. Sidebar shows the room, both clients in Insert mode at the input bar.
2. Andreas types `/qu` — input is **red**. Continues to `/quit` — input is **green**. Presses Enter → Andreas's TUI exits cleanly, terminal restored, server logs the disconnect.
3. Restart Andreas. Beth types `/me waves at @andreas` — green throughout. Enter → Beth's timeline shows `* beth waves at @andreas`. Andreas's timeline shows the same line, with the mention background highlight (pink bg + dark text).
4. Andreas types `/foo bar` — input is **red**. Presses Enter → input stays in the bar, no message sent, no system line. Backspaces and tries `/me hi` — green — Enter → both timelines show `* andreas hi`.
5. Andreas types `//me kicks` — input is **normal foreground** (escaped text). Enter → both timelines show a regular text message: `<andreas> /me kicks` (chevrons present, not action shape).
6. Andreas types `hello åse 🐧` — Enter → both timelines render the UTF-8 + emoji content correctly.
7. Andreas types `/clear` — green. Enter → Andreas's timeline empties; Beth's timeline is untouched.
8. Andreas types `/help` — green. Enter → Andreas sees five system lines in his timeline: "available commands:" header followed by four per-command lines (`  /quit — exit the client`, etc.). Beth sees nothing change in her timeline.
9. Andreas types `/help me` — green. Enter → Andreas sees a system line with `/me`'s `long_help`.
10. Stop the server (Ctrl+C in the server terminal). Andreas types `/me sad` — green. Enter → Andreas sees `-- system: not connected — /me requires an active connection` in his timeline. Beth's client also notices the disconnect and renders the existing 1.B "connection lost" surface.
11. UTF-8 cursor sanity: restart server and clients, Andreas types `ä😀x` — visually confirm the cursor sits at column 4 (1 + 2 + 1 cells), not column 6 (byte length).

If all eleven steps pass on a real terminal (not just headless tests), 1.C is ready to merge.

## Implementation discipline

- **TDD throughout.** Each task in the implementation plan starts with the failing test, makes it pass with the smallest code, refactors. Same pattern as 1.A and 1.B.
- **Single branch.** All work on `phase-1c`, branched from `main`. PR opens against `main` once the smoke test passes.
- **CI must be green** on Ubuntu (rustc latest stable; we discovered drift between local 1.93 and CI 1.95 during 1.B — assume it's still latest stable on CI).
- **No `Co-Authored-By` lines** in commit messages (per repo convention).
- **Conventional Commit prefixes:** `feat(commands):`, `fix(client):`, `test(commands):`, `feat(proto):`, `docs:`.

## Workspace version bump

Bump `[workspace.package].version` from `0.3.0` → `0.4.0`. Phase 1.C adds a new public crate API (`spaze-commands`'s public surface) and a new proto variant — both warrant a minor bump per semver-ish workspace convention we've used through Phase 1.

## Open questions / follow-ups for later phases

(Not blocking 1.C; captured here so they don't drift.)

- **`Buffer::push_local_system` placement.** This new method belongs on the `Buffer` enum / variant, but `RoomTimelineBuffer` and `HelpBuffer` have different storage. For 1.C, `HelpBuffer::push_local_system` can be a no-op (help buffer doesn't have a timeline; system lines from `/help` etc. should appear in the room buffer the user was on). Or we can disallow `/help` while in HelpBuffer entirely. Implementation plan should pick one — probably "system lines render on whatever buffer is active, and HelpBuffer just no-ops the push."
- **`/clear` while in HelpBuffer.** Same shape: no-op or refuse. Probably refuse (with a system line: "/clear has nothing to clear in this buffer"). Defer to plan.
- **Race condition on `apply_effect(SendActionMessage)` between connectivity check and `try_send`.** Connection state could flip between the check and the send. For 1.C, accept the race — `try_send` returning Err is logged but not surfaced to the user. Phase 2's reconnection logic will redesign this surface anyway.
- **Argument parsing for future commands.** When `/topic "with spaces in it"` lands in Phase 2, the tokenizer needs to grow. Likely a small `tokenize(&str) -> Vec<String>` that handles `"..."` and `\\` escapes. Out of scope for 1.C.
