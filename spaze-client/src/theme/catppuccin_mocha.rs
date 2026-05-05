//! Catppuccin Mocha — Spaze's default dark theme.
//!
//! Colors: <https://catppuccin.com/palette#mocha>

use ratatui::style::Color;

use super::Theme;

pub const CATPPUCCIN_MOCHA: Theme = Theme {
    // surfaces
    background: Color::Rgb(0x1e, 0x1e, 0x2e), // base
    surface: Color::Rgb(0x18, 0x18, 0x25),    // mantle
    overlay: Color::Rgb(0x31, 0x32, 0x44),    // surface0

    // text
    foreground: Color::Rgb(0xcd, 0xd6, 0xf4), // text
    muted: Color::Rgb(0x6c, 0x70, 0x86),      // overlay0
    subtle: Color::Rgb(0x45, 0x47, 0x5a),     // surface1

    // state
    primary: Color::Rgb(0xcb, 0xa6, 0xf7), // mauve
    success: Color::Rgb(0xa6, 0xe3, 0xa1), // green
    warning: Color::Rgb(0xf9, 0xe2, 0xaf), // yellow
    error: Color::Rgb(0xf3, 0x8b, 0xa8),   // red
    info: Color::Rgb(0x89, 0xb4, 0xfa),    // blue

    // chat-specific
    mention: Color::Rgb(0xf5, 0xc2, 0xe7), // pink (lighter)
    link: Color::Rgb(0x89, 0xdc, 0xeb),    // sky
    border: Color::Rgb(0x45, 0x47, 0x5a),  // surface1
    border_focused: Color::Rgb(0xcb, 0xa6, 0xf7), // mauve

    // tab kind tints
    tab_room: Color::Rgb(0x94, 0xe2, 0xd5),    // teal
    tab_dm: Color::Rgb(0xf9, 0xe2, 0xaf),      // yellow
    tab_special: Color::Rgb(0xfa, 0xb3, 0x87), // peach
    tab_unread: Color::Rgb(0xf3, 0x8b, 0xa8),  // pink

    syntect_theme_name: "Catppuccin Mocha",
};
