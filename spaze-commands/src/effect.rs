//! `Effect` — what command handlers return. The client interprets each
//! variant in `App::apply_effect`. Effects represent semantic intent, not
//! wire-level commands; the client constructs `ClientCommand`s from app state.

/// What command handlers return. Each variant is a piece of *semantic intent*
/// — translation to wire-level commands or App-state mutations happens in the
/// client's `apply_effect`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Quit the client cleanly.
    Quit,

    /// Empty the active buffer's local timeline. Session-only — no persistence
    /// concept yet (Phase 2 may revisit when scrollback exists).
    ClearActiveBuffer,

    /// Render a synthetic system message in the active buffer. Locally
    /// generated, never sent over the wire, never persisted. Visually
    /// identical to a server-issued `MessageBody::System`.
    SystemLine(String),

    /// Send an action message (`/me <content>`) to the server. The client
    /// constructs the full `PostMessage` from app state (room_id, author, etc.).
    SendActionMessage(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_variants_have_expected_shape() {
        // Verify the four 1.C variants exist and have the correct payloads.
        let _ = Effect::Quit;
        let _ = Effect::ClearActiveBuffer;
        let _ = Effect::SystemLine("usage: ...".into());
        let _ = Effect::SendActionMessage("waves".into());
    }

    #[test]
    fn effect_equality_works() {
        assert_eq!(Effect::Quit, Effect::Quit);
        assert_eq!(
            Effect::SystemLine("a".into()),
            Effect::SystemLine("a".into())
        );
        assert_ne!(
            Effect::SystemLine("a".into()),
            Effect::SystemLine("b".into())
        );
        assert_ne!(Effect::Quit, Effect::ClearActiveBuffer);
    }
}
