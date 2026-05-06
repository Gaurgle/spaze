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
}
