//! Client identity derivation.
//!
//! Phase 1.A: identity is purely deterministic from the `--name` flag.
//! `UserId` and `DeviceId` are derived via `UUIDv5` in a hardcoded namespace.
//! Phase 3 introduces real GitHub-backed identity and per-device Ed25519 keys.

use spaze_proto::{DeviceId, UserId};
use uuid::{Uuid, uuid};

/// Hardcoded namespace UUID for Spaze identity derivation.
///
/// Generated once via `uuidgen` for this project. Stable across all releases —
/// changing it would re-shuffle every name's derived `UserId`.
const SPAZE_NAMESPACE: Uuid = uuid!("8e7a9f1c-3d52-4e8b-9c1d-7f6a8b9c0d1e");

/// Derive `(UserId, DeviceId)` from a display name.
///
/// In Phase 1.A, `DeviceId == UserId` for the same name — there's no real
/// device-vs-user distinction yet. Phase 3 splits them when device keypairs
/// land.
#[must_use]
pub fn derive_identity(name: &str) -> (UserId, DeviceId) {
    let user_uuid = Uuid::new_v5(&SPAZE_NAMESPACE, name.as_bytes());
    (UserId::from_uuid(user_uuid), DeviceId::from_uuid(user_uuid))
}
