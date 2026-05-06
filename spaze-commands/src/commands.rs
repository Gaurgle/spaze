//! The four 1.C command handlers: `/quit`, `/help`, `/me`, `/clear`.
//! Each is a pure function of `(args, ctx) -> Vec<Effect>`.

use crate::effect::Effect;
use crate::registry::HandlerContext;

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
}
