//! Command registry: `Command` struct, `HandlerContext`, `Handler` typedef,
//! the static `REGISTRY` slice, and `lookup` for name/alias resolution.

use crate::effect::Effect;

/// Function signature for command handlers. Pure: no I/O, no mutation,
/// no async. Returns a list of effects the client will apply in order.
pub type Handler = fn(args: &[&str], ctx: &HandlerContext) -> Vec<Effect>;

/// Context passed to every handler. Carries the registry so `/help` can
/// introspect available commands. Future fields (current room name,
/// connection state, etc.) land here additively.
#[derive(Debug, Clone, Copy)]
pub struct HandlerContext<'a> {
    pub registry: &'a [Command],
}

/// One command in the registry. Lives in `&'static` data — entries are
/// const-constructible and shared across the process.
#[derive(Debug, Clone, Copy)]
pub struct Command {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub short_help: &'static str,
    pub long_help: &'static str,
    pub handler: Handler,
}

/// Look up a command by canonical name OR alias. Case-sensitive.
#[must_use]
pub fn lookup<'a>(name: &str, registry: &'a [Command]) -> Option<&'a Command> {
    registry
        .iter()
        .find(|c| c.name == name || c.aliases.contains(&name))
}

use crate::commands::{handle_clear, handle_help, handle_me, handle_quit};

/// The static slash command registry. Adding a command in 1.C+ is one new
/// entry here plus the handler in `commands.rs`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;

    fn dummy_handler(_args: &[&str], _ctx: &HandlerContext) -> Vec<Effect> {
        vec![Effect::Quit]
    }

    fn fixture() -> [Command; 2] {
        [
            Command {
                name: "quit",
                aliases: &["q"],
                short_help: "exit",
                long_help: "/quit — exit the client",
                handler: dummy_handler,
            },
            Command {
                name: "help",
                aliases: &["h", "?cmd"],
                short_help: "help",
                long_help: "/help — show help",
                handler: dummy_handler,
            },
        ]
    }

    #[test]
    fn lookup_finds_by_canonical_name() {
        let reg = fixture();
        let cmd = lookup("quit", &reg).expect("quit should be found");
        assert_eq!(cmd.name, "quit");
    }

    #[test]
    fn lookup_finds_by_alias() {
        let reg = fixture();
        let cmd = lookup("q", &reg).expect("q alias should resolve");
        assert_eq!(cmd.name, "quit");
        let cmd = lookup("?cmd", &reg).expect("?cmd alias should resolve");
        assert_eq!(cmd.name, "help");
    }

    #[test]
    fn lookup_returns_none_on_miss() {
        let reg = fixture();
        assert!(lookup("nope", &reg).is_none());
    }

    #[test]
    fn lookup_is_case_sensitive() {
        // Slash commands are ASCII canonical-case. /Quit doesn't match /quit.
        let reg = fixture();
        assert!(lookup("Quit", &reg).is_none());
    }

    #[test]
    fn handler_context_holds_registry_reference() {
        let reg = fixture();
        let ctx = HandlerContext { registry: &reg };
        assert_eq!(ctx.registry.len(), 2);
    }

    #[test]
    fn real_registry_has_four_commands() {
        // Sanity: 1.C ships exactly /quit, /help, /me, /clear.
        assert_eq!(REGISTRY.len(), 4);
        let names: Vec<_> = REGISTRY.iter().map(|c| c.name).collect();
        assert!(names.contains(&"quit"));
        assert!(names.contains(&"help"));
        assert!(names.contains(&"me"));
        assert!(names.contains(&"clear"));
    }

    #[test]
    fn real_registry_no_duplicate_names() {
        let mut names: Vec<_> = REGISTRY.iter().map(|c| c.name).collect();
        names.sort_unstable();
        let dedup_len = {
            let mut d = names.clone();
            d.dedup();
            d.len()
        };
        assert_eq!(names.len(), dedup_len, "duplicate command names: {names:?}");
    }

    #[test]
    fn real_registry_aliases_dont_collide() {
        // No alias may equal another command's canonical name or another
        // command's alias.
        for cmd in REGISTRY {
            for alias in cmd.aliases {
                // Not equal to any name except its own command's name.
                for other in REGISTRY {
                    if other.name == cmd.name {
                        continue;
                    }
                    assert_ne!(
                        *alias, other.name,
                        "alias {alias:?} of /{} collides with name /{}",
                        cmd.name, other.name
                    );
                    for other_alias in other.aliases {
                        assert_ne!(
                            alias, other_alias,
                            "alias {alias:?} of /{} collides with alias {other_alias:?} of /{}",
                            cmd.name, other.name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn real_registry_every_command_has_help_text() {
        for cmd in REGISTRY {
            assert!(!cmd.name.is_empty(), "command has empty name");
            assert!(
                !cmd.short_help.is_empty(),
                "/{} has empty short_help",
                cmd.name
            );
            assert!(
                !cmd.long_help.is_empty(),
                "/{} has empty long_help",
                cmd.name
            );
        }
    }
}
