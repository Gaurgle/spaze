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

    /// Name passed to syntect's loaded `ThemeSet` for code-block highlighting.
    /// Phase 4 must bundle a matching `.tmTheme` file — Catppuccin themes
    /// are NOT in syntect's default theme set, so the loader will need to
    /// pull one in via `ThemeSet::load_from_reader` from a bundled asset.
    pub syntect_theme_name: &'static str,
}
