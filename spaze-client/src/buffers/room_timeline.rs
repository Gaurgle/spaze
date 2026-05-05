//! `RoomTimelineBuffer` — chat timeline for a single Room (or DM).

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
        Self {
            offset_from_bottom: 0,
            stuck_to_bottom: true,
        }
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
    pub fn new(
        room_id: RoomId,
        display_name: String,
        kind: RoomKind,
        self_display_name: String,
    ) -> Self {
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

    #[must_use]
    pub fn title(&self) -> &str {
        &self.display_name
    }

    #[must_use]
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
