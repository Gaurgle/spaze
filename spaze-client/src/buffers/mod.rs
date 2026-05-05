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
    #[must_use]
    pub fn title(&self) -> &str {
        match self {
            Buffer::Room(b) => b.title(),
            Buffer::Help(b) => b.title(),
        }
    }

    #[must_use]
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
