//! Protocol-level error type sent over the wire.
//!
//! This is intentionally small and stable. Internal server errors
//! (database failures, panics, etc.) are surfaced to clients as
//! [`ProtocolError::InternalError`] without leaking detail.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum ProtocolError {
    #[error("not authenticated")]
    NotAuthenticated,

    #[error("permission denied")]
    PermissionDenied,

    #[error("not found: {what}")]
    NotFound { what: String },

    #[error("invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("rate limited; retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u32 },

    #[error("internal server error")]
    InternalError,
}
