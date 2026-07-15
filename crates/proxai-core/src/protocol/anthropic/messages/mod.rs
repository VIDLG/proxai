//! Anthropic Messages protocol-native helpers and schema behavior.

pub(crate) mod wire;

// Expose protocol types through one stable facade instead of duplicating the
// public API under a nested `wire` namespace.
pub use wire::*;
