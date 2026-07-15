//! Carrier-independent protocol models and cross-protocol translation.
//!
//! The core accepts structured JSON values or structured stream events. HTTP,
//! SSE byte framing, provider transport, routing, logging, capture, and
//! diagnostics belong to the application crate.

pub mod protocol;
pub mod translation;
