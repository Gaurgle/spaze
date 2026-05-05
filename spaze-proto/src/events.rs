//! Wire frames exchanged between client and server.
//!
//! Top-level frame types are [`ClientFrame`] (client → server) and
//! [`ServerFrame`] (server → client). Client frames always carry a
//! [`RequestId`] so the matching [`ServerFrame::Response`] can be
//! correlated back. Unsolicited pushes (typing, new message, user joined)
//! arrive as [`ServerFrame::Event`].
//!
//! All variants are serde-tagged unions; the wire format is JSON. Keep
//! enums under careful protocol versioning — see [`crate::PROTOCOL_VERSION`].

use serde::{Deserialize, Serialize};

use crate::error::ProtocolError;
use crate::ids::{DeviceId, MessageId, RoomId, UserId};
use crate::messages::{Message, MessageBody};

/// Per-connection correlation token for client commands and their responses.
///
/// The client increments a counter for each command sent on a given
/// `WebSocket` session. `RequestIds` are unique within a single connection,
/// not globally — they are not persisted server-side beyond the response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(pub u64);

/// Top-level frame sent client → server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientFrame {
    pub request_id: RequestId,
    pub command: ClientCommand,
}

/// Top-level frame sent server → client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum ServerFrame {
    /// Reply to a specific [`ClientFrame`]. `request_id` matches the original.
    Response {
        request_id: RequestId,
        result: CommandOutcome,
    },
    /// Unsolicited server-pushed event (no matching request).
    Event(ServerEvent),
}

/// Outcome of a client command. Explicit enum (rather than `Result`) so the
/// JSON shape is `{"outcome": "ok", "payload": ...}` / `{"outcome": "err", "error": ...}`
/// instead of serde's default `{"Ok": ...}` / `{"Err": ...}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum CommandOutcome {
    Ok { payload: ResponsePayload },
    Err { error: ProtocolError },
}

/// Successful response payloads. Most commands respond with [`ResponsePayload::Empty`];
/// commands that return data attach it via a dedicated variant.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResponsePayload {
    /// The command succeeded with no return data.
    Empty,
    /// `PostMessage` succeeded — server returns the canonical [`Message`]
    /// (with server-assigned id and timestamp) for the client to swap in.
    MessagePosted(Message),
}

/// Server-pushed events (not direct responses to client commands).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    /// A new message was posted in a room the client is subscribed to.
    MessagePosted(Message),
    /// A previously-posted message was edited.
    MessageEdited {
        message_id: MessageId,
        body: MessageBody,
        edited_at_ms: i64,
    },
    /// A previously-posted message was deleted.
    MessageDeleted {
        message_id: MessageId,
        deleted_at_ms: i64,
    },
    /// A user joined a room.
    UserJoined { user_id: UserId, room_id: RoomId },
    /// A user left a room.
    UserLeft { user_id: UserId, room_id: RoomId },
    /// Typing indicator (best-effort fan-out).
    Typing { user_id: UserId, room_id: RoomId },
}

/// Client commands carried inside a [`ClientFrame`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientCommand {
    PostMessage {
        room_id: RoomId,
        author_id: UserId,
        author_device_id: DeviceId,
        author_display_name: String,
        body: MessageBody,
    },
    EditMessage {
        message_id: MessageId,
        body: MessageBody,
    },
    DeleteMessage {
        message_id: MessageId,
    },
    JoinRoom {
        room_id: RoomId,
    },
    LeaveRoom {
        room_id: RoomId,
    },
    StartTyping {
        room_id: RoomId,
    },
}
