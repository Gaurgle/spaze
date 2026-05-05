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
}
