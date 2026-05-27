//! App state machine — top-level state for the TUI client.

use spaze_commands::Effect;
use spaze_proto::{DeviceId, Message, MessageBody, MessageId, RoomId, UserId};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedRegion {
    Sidebar,
    Tabs,
    Buffer,
    Input,
    // Future: Popup (Phase 2's first-run assistant)
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
    pub focus: FocusedRegion,
    pub sidebar_selected: Option<usize>,
    pub last_layout: Option<crate::tui::LayoutRects>,
}

impl App {
    /// Construct initial state with the hardcoded room and a help buffer.
    #[must_use]
    pub fn new(identity: Identity, _server_url: String) -> Self {
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
            focus: FocusedRegion::Sidebar,
            sidebar_selected: Some(0),
            last_layout: None,
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
        self.buffers.iter().position(pred)
    }

    /// Set the focused region, maintaining the invariant:
    /// `focus == Input` iff `mode == Insert`.
    ///
    /// Setting `Input` enters Insert mode (mouse-click-on-input shortcut).
    /// Setting non-Input from Insert mode returns to Normal (click elsewhere).
    pub fn set_focus(&mut self, region: FocusedRegion) {
        self.focus = region;
        match region {
            FocusedRegion::Input => self.mode = InputMode::Insert,
            _ => {
                if matches!(self.mode, InputMode::Insert) {
                    self.mode = InputMode::Normal;
                }
            }
        }
    }

    /// Move sidebar selection to the previous Room buffer (wraps around).
    /// No-op if there are zero or one selectable items.
    pub fn sidebar_move_up(&mut self) {
        let selectable: Vec<usize> = self
            .buffers
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(b, Buffer::Room(_)))
            .map(|(i, _)| i)
            .collect();
        if selectable.is_empty() {
            self.sidebar_selected = None;
            return;
        }
        let cur = self.sidebar_selected.unwrap_or(selectable[0]);
        let pos = selectable.iter().position(|&i| i == cur).unwrap_or(0);
        let prev = if pos == 0 {
            selectable[selectable.len() - 1]
        } else {
            selectable[pos - 1]
        };
        self.sidebar_selected = Some(prev);
    }

    /// Move sidebar selection to the next Room buffer (wraps around).
    /// No-op if there are zero or one selectable items.
    pub fn sidebar_move_down(&mut self) {
        let selectable: Vec<usize> = self
            .buffers
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(b, Buffer::Room(_)))
            .map(|(i, _)| i)
            .collect();
        if selectable.is_empty() {
            self.sidebar_selected = None;
            return;
        }
        let cur = self.sidebar_selected.unwrap_or(selectable[0]);
        let pos = selectable.iter().position(|&i| i == cur).unwrap_or(0);
        let next = selectable[(pos + 1) % selectable.len()];
        self.sidebar_selected = Some(next);
    }

    /// Activate the currently selected sidebar item (switch active tab).
    /// No-op if `sidebar_selected == None`.
    pub fn sidebar_activate(&mut self) {
        if let Some(idx) = self.sidebar_selected {
            if idx < self.buffers.len() {
                self.active = idx;
            }
        }
    }

    /// Apply a command effect. The single mutation point for command-driven
    /// state changes. The wire-level send for `SendActionMessage` is *not*
    /// performed here — it requires async access to the WS sink, so the
    /// caller (`handle_key`) does that part. This method covers the
    /// synchronous, App-state-only effects: `Quit`, `ClearActiveBuffer`,
    /// `SystemLine`. For `SendActionMessage`, see `handle_key`'s Insert-mode
    /// branch in `lib.rs`.
    pub fn apply_effect(&mut self, effect: Effect) {
        match effect {
            Effect::Quit => {
                self.should_quit = true;
            }
            Effect::ClearActiveBuffer => {
                if let Some(Buffer::Room(rb)) = self.buffers.get_mut(self.active) {
                    rb.messages.clear();
                    rb.scroll.to_bottom();
                }
            }
            Effect::SystemLine(content) => {
                self.push_local_system(content);
            }
            Effect::SendActionMessage(_) => {
                // Wire-level effect; handled in `handle_key` where the WS
                // sink is in scope. Reaching here means the caller forgot
                // to filter — log and drop, don't panic.
                tracing::warn!(
                    "App::apply_effect received SendActionMessage; \
                     should be handled by caller before dispatch"
                );
            }
        }
    }

    /// Push a synthetic `MessageBody::System` message onto the active buffer
    /// (whichever it is). Locally generated, never sent over the wire,
    /// never persisted.
    fn push_local_system(&mut self, content: String) {
        let msg = Message {
            id: MessageId::from_uuid(Uuid::nil()),
            room_id: RoomId::from_uuid(Uuid::nil()),
            author_id: UserId::from_uuid(Uuid::nil()),
            author_device_id: DeviceId::from_uuid(Uuid::nil()),
            author_display_name: "spaze".into(),
            created_at_ms: chrono::Utc::now().timestamp_millis(),
            edited_at_ms: None,
            deleted_at_ms: None,
            body: MessageBody::System { content },
        };
        // For 1.C, system lines always render on the active room buffer if
        // one exists; HelpBuffer doesn't have a timeline.
        if let Some(Buffer::Room(rb)) = self.buffers.get_mut(self.active) {
            rb.push_message(msg);
            return;
        }
        // Active buffer isn't a room — fall back to the first room buffer.
        for buf in &mut self.buffers {
            if let Buffer::Room(rb) = buf {
                rb.push_message(msg);
                return;
            }
        }
    }
}
