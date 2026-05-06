//! Input parsing: `classify_input` (per-keystroke, zero-alloc) and
//! `parse_input` (per-Enter, allocates owned strings).

use crate::registry::Command;

/// Per-keystroke classification of the input buffer. Allocates nothing —
/// the `name` field borrows from the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputClass<'a> {
    /// Plain text (no `/` prefix, or whitespace before the prefix).
    Text,
    /// Escaped text (input started with `//`). Will be sent as text with one
    /// leading slash stripped.
    EscapedText,
    /// First token after `/` matched a registry entry by name or alias.
    ValidCommand { name: &'a str },
    /// First token after `/` did NOT match. Includes bare `/` (empty name).
    InvalidCommand { name: &'a str },
}

/// Classify the input buffer for the live-coloring path. Cheap enough to
/// call on every keystroke.
#[must_use]
pub fn classify_input<'a>(input: &'a str, registry: &[Command]) -> InputClass<'a> {
    if input.starts_with("//") {
        return InputClass::EscapedText;
    }
    let Some(rest) = input.strip_prefix('/') else {
        return InputClass::Text;
    };
    let name = rest.split_whitespace().next().unwrap_or("");
    if name.is_empty() {
        return InputClass::InvalidCommand { name };
    }
    if registry
        .iter()
        .any(|c| c.name == name || c.aliases.contains(&name))
    {
        InputClass::ValidCommand { name }
    } else {
        InputClass::InvalidCommand { name }
    }
}

/// Result of parsing the input buffer at Enter time. Owned strings so the
/// caller can clear `app.input_buffer` immediately after dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputKind {
    /// A plain text message. Send the content as `MessageBody::Text`.
    Text(String),

    /// An escaped text message (input started with `//`). The leading slash
    /// has been stripped — send the content as `MessageBody::Text`.
    EscapedText(String),

    /// A slash command. `name` is the first whitespace-delimited token after
    /// the `/`; may be empty for bare `/`. `args` is the remaining tokens.
    Command { name: String, args: Vec<String> },
}

/// Parse the input buffer into an `InputKind` for Enter-time dispatch.
///
/// Rules:
/// - `"//..."` → `EscapedText` with one leading slash stripped.
/// - `"/..."`  → `Command { name, args }` (name may be empty for bare `/`).
/// - else      → `Text(input)`.
#[must_use]
pub fn parse_input(input: &str) -> InputKind {
    if input.starts_with("//") {
        // Strip ONE slash. "//me" → "/me" as text. "///x" → "//x" as text.
        return InputKind::EscapedText(input[1..].to_string());
    }
    if let Some(rest) = input.strip_prefix('/') {
        let mut tokens = rest.split_whitespace();
        let name = tokens.next().unwrap_or("").to_string();
        let args: Vec<String> = tokens.map(String::from).collect();
        return InputKind::Command { name, args };
    }
    InputKind::Text(input.to_string())
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn empty_input_is_text() {
        assert_eq!(parse_input(""), InputKind::Text(String::new()));
    }

    #[test]
    fn plain_text_is_text() {
        assert_eq!(parse_input("hello world"), InputKind::Text("hello world".into()));
    }

    #[test]
    fn slash_mid_line_is_text() {
        assert_eq!(
            parse_input("hello /world"),
            InputKind::Text("hello /world".into())
        );
    }

    #[test]
    fn double_slash_is_escaped_text_with_one_slash_stripped() {
        assert_eq!(
            parse_input("//me kicks"),
            InputKind::EscapedText("/me kicks".into())
        );
    }

    #[test]
    fn triple_slash_is_escaped_text_with_one_slash_stripped() {
        assert_eq!(
            parse_input("///me"),
            InputKind::EscapedText("//me".into())
        );
    }

    #[test]
    fn single_slash_is_command_with_empty_name() {
        assert_eq!(
            parse_input("/"),
            InputKind::Command {
                name: String::new(),
                args: Vec::new(),
            }
        );
    }

    #[test]
    fn command_no_args() {
        assert_eq!(
            parse_input("/quit"),
            InputKind::Command {
                name: "quit".into(),
                args: Vec::new(),
            }
        );
    }

    #[test]
    fn command_with_args() {
        assert_eq!(
            parse_input("/me kicks the build"),
            InputKind::Command {
                name: "me".into(),
                args: vec!["kicks".into(), "the".into(), "build".into()],
            }
        );
    }

    #[test]
    fn command_collapses_internal_whitespace() {
        // split_whitespace collapses runs of any whitespace.
        assert_eq!(
            parse_input("/me  hi   there"),
            InputKind::Command {
                name: "me".into(),
                args: vec!["hi".into(), "there".into()],
            }
        );
    }

    #[test]
    fn command_with_utf8_args() {
        assert_eq!(
            parse_input("/me waves at åse 🐧"),
            InputKind::Command {
                name: "me".into(),
                args: vec!["waves".into(), "at".into(), "åse".into(), "🐧".into()],
            }
        );
    }
}

#[cfg(test)]
mod classify_tests {
    use super::*;
    use crate::registry::{Command, HandlerContext};
    use crate::effect::Effect;

    fn fixture_handler(_a: &[&str], _c: &HandlerContext) -> Vec<Effect> {
        vec![]
    }

    fn fixture() -> [Command; 2] {
        [
            Command {
                name: "quit",
                aliases: &["q"],
                short_help: "x",
                long_help: "x",
                handler: fixture_handler,
            },
            Command {
                name: "me",
                aliases: &[],
                short_help: "x",
                long_help: "x",
                handler: fixture_handler,
            },
        ]
    }

    #[test]
    fn empty_is_text() {
        assert_eq!(classify_input("", &fixture()), InputClass::Text);
    }

    #[test]
    fn plain_text_is_text() {
        assert_eq!(classify_input("hello", &fixture()), InputClass::Text);
    }

    #[test]
    fn slash_mid_line_is_text() {
        assert_eq!(
            classify_input("hello /world", &fixture()),
            InputClass::Text
        );
    }

    #[test]
    fn whitespace_before_slash_is_text() {
        assert_eq!(
            classify_input("   /quit", &fixture()),
            InputClass::Text
        );
    }

    #[test]
    fn double_slash_is_escaped_text() {
        assert_eq!(
            classify_input("//me kicks", &fixture()),
            InputClass::EscapedText
        );
    }

    #[test]
    fn registry_hit_by_name_is_valid() {
        assert_eq!(
            classify_input("/quit", &fixture()),
            InputClass::ValidCommand { name: "quit" }
        );
    }

    #[test]
    fn registry_hit_by_alias_is_valid() {
        assert_eq!(
            classify_input("/q", &fixture()),
            InputClass::ValidCommand { name: "q" }
        );
    }

    #[test]
    fn registry_hit_with_args_uses_command_name_only() {
        assert_eq!(
            classify_input("/me kicks the build", &fixture()),
            InputClass::ValidCommand { name: "me" }
        );
    }

    #[test]
    fn registry_miss_is_invalid() {
        assert_eq!(
            classify_input("/foo", &fixture()),
            InputClass::InvalidCommand { name: "foo" }
        );
    }

    #[test]
    fn bare_slash_is_invalid_with_empty_name() {
        assert_eq!(
            classify_input("/", &fixture()),
            InputClass::InvalidCommand { name: "" }
        );
    }

    #[test]
    fn slash_with_only_whitespace_is_invalid_with_empty_name() {
        assert_eq!(
            classify_input("/   ", &fixture()),
            InputClass::InvalidCommand { name: "" }
        );
    }

    #[test]
    fn utf8_command_name_misses_registry_and_is_invalid() {
        assert_eq!(
            classify_input("/qüit", &fixture()),
            InputClass::InvalidCommand { name: "qüit" }
        );
    }
}
