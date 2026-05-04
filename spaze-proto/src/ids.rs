//! Newtype-wrapped `UUIDv7` IDs for the Spaze wire protocol.
//!
//! `UUIDv7` is time-ordered, so values sort lexically by creation time.
//! That property matters for chat (message ordering) and for `SQLite`
//! primary-key locality.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
            Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        #[allow(clippy::new_without_default)]
        impl $name {
            /// Generate a fresh UUIDv7-backed identifier.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// Wrap an existing UUID.
            #[must_use]
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Unwrap to the underlying UUID.
            #[must_use]
            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(
    /// Identifies a Spaze user (one human, possibly many devices).
    UserId
);
id_type!(
    /// Identifies a single registered client device for a user.
    DeviceId
);
id_type!(
    /// Identifies a Space — a project workspace inside a server.
    /// One `spaze-server` daemon may host multiple Spaces.
    SpaceId
);
id_type!(
    /// Identifies a Room — a conversation channel inside a Space.
    /// DMs are Rooms with `kind = direct` and exactly two members.
    RoomId
);
id_type!(
    /// Identifies a single message.
    MessageId
);
