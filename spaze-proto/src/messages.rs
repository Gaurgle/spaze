//! Chat message types.
//!
//! [`Message`] is the durable, server-authoritative shape stored in `SQLite`
//! and broadcast to room subscribers. [`MessageBody`] is intentionally
//! a small enum: text and system today; attachments, code blocks, and
//! reactions are added in later phases.
//!
//! `edited_at_ms` and `deleted_at_ms` are `Option<i64>` on the [`Message`]
//! itself — server-assigned, easy to query, and avoid baking state into
//! [`MessageBody`].

use serde::{Deserialize, Serialize};

use crate::ids::{DeviceId, MessageId, RoomId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub room_id: RoomId,
    pub author_id: UserId,
    pub author_device_id: DeviceId,
    /// Unix timestamp in milliseconds (server-assigned at post time).
    pub created_at_ms: i64,
    /// Server-assigned timestamp of the most recent edit, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_at_ms: Option<i64>,
    /// Server-assigned timestamp of soft-delete, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at_ms: Option<i64>,
    pub body: MessageBody,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MessageBody {
    /// A plain user-authored text message.
    Text { content: String },
    /// A server-generated system event rendered inline (joins, renames, etc.).
    System { content: String },
}
