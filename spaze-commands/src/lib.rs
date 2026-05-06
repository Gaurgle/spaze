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

// NOTE: pub use lines commented out pending Tasks 4-10 which add the symbols.
// Uncomment as each module gains its implementation:
// - effect::{Effect} — Task 4
// - parser::{InputClass, InputKind, classify_input, parse_input} — Task 5
// - registry::{Command, Handler, HandlerContext, REGISTRY, lookup} — Task 6
//
pub use effect::Effect;
pub use parser::{InputKind, parse_input};
pub use registry::{Command, Handler, HandlerContext, lookup};
// pub use registry::REGISTRY;
