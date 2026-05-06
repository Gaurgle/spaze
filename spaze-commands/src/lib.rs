//! `spaze-commands` — slash command registry, parser, and effect model.
//!
//! This crate is intentionally zero-dependency (std-only). Commands are pure
//! functions of `(args, ctx) -> Vec<Effect>`. The client interprets effects.
//!
//! ## Public surface
//!
//! - [`Effect`] — what handlers return (semantic intent, not wire-level).
//! - [`parse_input`] / [`InputKind`] — Enter-time parser (allocates).
//! - [`classify_input`] / [`InputClass`] — keystroke-time classifier (zero alloc).
//! - [`Command`] / [`HandlerContext`] / [`Handler`] — registry types.
//! - [`lookup`] / [`REGISTRY`] — registry access.

pub mod commands;
pub mod effect;
pub mod parser;
pub mod registry;

pub use effect::Effect;
pub use parser::{InputClass, InputKind, classify_input, parse_input};
pub use registry::{Command, Handler, HandlerContext, REGISTRY, lookup};
