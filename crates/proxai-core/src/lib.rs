//! Carrier-independent ingress, routing, protocol models, and translation.
//!
//! The core accepts structured configuration, JSON values, or structured stream
//! events. HTTP, SSE byte framing, provider transport, concrete observation
//! sinks, logging, capture, and diagnostics belong to the application crate.

pub mod error;
pub mod ingress;
mod json;
pub mod observe;
pub mod protocol;
pub mod routing;
pub mod translation;
