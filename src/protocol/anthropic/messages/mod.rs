//! Anthropic Messages protocol-native helpers and schema behavior.

pub mod wire;

// Re-export everything from protocol at pub level so tests can access the full schema.
pub use wire::*;
