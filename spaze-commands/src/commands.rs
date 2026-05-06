//! The four 1.C command handlers: `/quit`, `/help`, `/me`, `/clear`.
//! Each is a pure function of `(args, ctx) -> Vec<Effect>`.

use crate::effect::Effect;
use crate::registry::{HandlerContext, lookup};

/// `/quit` — exit the client.
#[must_use]
pub fn handle_quit(_args: &[&str], _ctx: &HandlerContext) -> Vec<Effect> {
    vec![Effect::Quit]
}

/// `/clear` — empty the active buffer's local timeline.
#[must_use]
pub fn handle_clear(_args: &[&str], _ctx: &HandlerContext) -> Vec<Effect> {
    vec![Effect::ClearActiveBuffer]
}

/// `/me <action text>` — send the rest of the line as an action message.
#[must_use]
pub fn handle_me(args: &[&str], _ctx: &HandlerContext) -> Vec<Effect> {
    if args.is_empty() {
        return vec![Effect::SystemLine("usage: /me <action text>".into())];
    }
    vec![Effect::SendActionMessage(args.join(" "))]
}

/// `/help` — list commands. `/help <name>` — show `long_help` for one.
///
/// Output is one `Effect::SystemLine` per visual line. Each line becomes its
/// own synthetic Message in the buffer; we don't embed `\n` and rely on the
/// renderer to split (the room timeline render path treats one Message as
/// one Line).
#[must_use]
pub fn handle_help(args: &[&str], ctx: &HandlerContext) -> Vec<Effect> {
    if args.is_empty() {
        let mut effects = vec![Effect::SystemLine("available commands:".into())];
        for cmd in ctx.registry {
            effects.push(Effect::SystemLine(format!(
                "  /{} — {}",
                cmd.name, cmd.short_help
            )));
        }
        return effects;
    }
    // Strip a leading slash if the user typed `/help /me`.
    let target = args[0].trim_start_matches('/');
    match lookup(target, ctx.registry) {
        Some(cmd) => vec![Effect::SystemLine(format!("/{}: {}", cmd.name, cmd.long_help))],
        None => vec![Effect::SystemLine(format!("unknown command: /{target}"))],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;
    use crate::registry::HandlerContext;

    fn empty_ctx() -> HandlerContext<'static> {
        HandlerContext { registry: &[] }
    }

    #[test]
    fn quit_returns_quit_effect() {
        assert_eq!(handle_quit(&[], &empty_ctx()), vec![Effect::Quit]);
    }

    #[test]
    fn quit_ignores_args() {
        assert_eq!(
            handle_quit(&["with", "ignored", "args"], &empty_ctx()),
            vec![Effect::Quit]
        );
    }

    #[test]
    fn clear_returns_clear_effect() {
        assert_eq!(
            handle_clear(&[], &empty_ctx()),
            vec![Effect::ClearActiveBuffer]
        );
    }

    #[test]
    fn me_with_no_args_returns_usage_systemline() {
        assert_eq!(
            handle_me(&[], &empty_ctx()),
            vec![Effect::SystemLine("usage: /me <action text>".into())]
        );
    }

    #[test]
    fn me_with_one_arg_sends_action_message() {
        assert_eq!(
            handle_me(&["waves"], &empty_ctx()),
            vec![Effect::SendActionMessage("waves".into())]
        );
    }

    #[test]
    fn me_joins_multiple_args_with_single_space() {
        assert_eq!(
            handle_me(&["kicks", "the", "build"], &empty_ctx()),
            vec![Effect::SendActionMessage("kicks the build".into())]
        );
    }

    #[test]
    fn me_preserves_utf8_in_joined_content() {
        assert_eq!(
            handle_me(&["waves", "at", "åse", "🐧"], &empty_ctx()),
            vec![Effect::SendActionMessage("waves at åse 🐧".into())]
        );
    }

    use crate::registry::Command;

    fn fixture_registry() -> [Command; 2] {
        [
            Command {
                name: "quit",
                aliases: &["q"],
                short_help: "exit the client",
                long_help: "/quit — exit cleanly",
                handler: handle_quit,
            },
            Command {
                name: "me",
                aliases: &[],
                short_help: "send an action message",
                long_help: "/me <text> — send action",
                handler: handle_me,
            },
        ]
    }

    #[test]
    fn help_no_args_lists_all_commands_with_header() {
        let reg = fixture_registry();
        let ctx = HandlerContext { registry: &reg };
        let effects = handle_help(&[], &ctx);
        // Header + one line per command = 1 + 2 = 3 effects.
        assert_eq!(effects.len(), 3);
        assert_eq!(
            effects[0],
            Effect::SystemLine("available commands:".into())
        );
        assert_eq!(
            effects[1],
            Effect::SystemLine("  /quit — exit the client".into())
        );
        assert_eq!(
            effects[2],
            Effect::SystemLine("  /me — send an action message".into())
        );
    }

    #[test]
    fn help_with_known_name_returns_long_help() {
        let reg = fixture_registry();
        let ctx = HandlerContext { registry: &reg };
        let effects = handle_help(&["me"], &ctx);
        assert_eq!(
            effects,
            vec![Effect::SystemLine("/me: /me <text> — send action".into())]
        );
    }

    #[test]
    fn help_strips_leading_slash_from_arg() {
        let reg = fixture_registry();
        let ctx = HandlerContext { registry: &reg };
        let effects = handle_help(&["/me"], &ctx);
        assert_eq!(
            effects,
            vec![Effect::SystemLine("/me: /me <text> — send action".into())]
        );
    }

    #[test]
    fn help_with_unknown_name_returns_unknown_command() {
        let reg = fixture_registry();
        let ctx = HandlerContext { registry: &reg };
        let effects = handle_help(&["nope"], &ctx);
        assert_eq!(
            effects,
            vec![Effect::SystemLine("unknown command: /nope".into())]
        );
    }
}
