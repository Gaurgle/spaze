//! Frame rendering for the Phase 1.A stdin/stdout client.
//!
//! Phase 1.B replaces these with TUI widgets.

use spaze_proto::{
    CommandOutcome, Message, MessageBody, ResponsePayload, ServerEvent, ServerFrame,
};

/// Render a `ServerFrame` to stdout/stderr.
///
/// - `Response::Ok(MessagePosted)` and `Event(MessagePosted)` both render the
///   message line.
/// - `Response::Err` writes to stderr.
/// - Other variants are no-ops in 1.A.
pub fn render_frame(frame: &ServerFrame) {
    match frame {
        ServerFrame::Response { result, .. } => match result {
            CommandOutcome::Ok {
                payload: ResponsePayload::MessagePosted(msg),
            } => print_message(msg),
            // `ResponsePayload::Empty` and any future non-exhaustive variants:
            // no-op in 1.A.
            CommandOutcome::Ok { .. } => {}
            CommandOutcome::Err { error } => eprintln!("error: {error}"),
        },
        ServerFrame::Event(ServerEvent::MessagePosted(msg)) => print_message(msg),
        ServerFrame::Event(_) => {
            // Other events (UserJoined, Typing, etc.) deferred to Phase 2.
        }
    }
}

/// Print a single message in the 1.A "<`short_uuid`> body" format.
pub fn print_message(msg: &Message) {
    let short = format!("{:.8}", msg.author_id.as_uuid().simple());
    match &msg.body {
        MessageBody::Text { content } => println!("<{short}> {content}"),
        MessageBody::System { content } => println!("-- system: {content}"),
        _ => {
            // Future MessageBody variants — no-op in 1.A.
        }
    }
}
