//! Time helpers. Centralized so the rest of the server doesn't sprinkle `SystemTime` plumbing.

use std::time::{SystemTime, UNIX_EPOCH};

/// Current unix timestamp in milliseconds.
///
/// # Panics
///
/// Panics if the system clock is set before the unix epoch (1970-01-01),
/// which would indicate a deeply broken machine.
#[must_use]
pub fn now_unix_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before unix epoch")
            .as_millis(),
    )
    .expect("timestamp does not fit in i64")
}
