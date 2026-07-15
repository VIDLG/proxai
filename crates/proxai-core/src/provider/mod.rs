//! Carrier-independent provider protocol preparation and compatibility handling.
//!
//! This module operates on structured JSON values and stream events only. HTTP
//! transport, authentication, byte framing, timeouts, and concrete observation
//! sinks belong to downstream composition.

mod normalizer;
mod request;
mod response;

pub use normalizer::{ProviderCompatibility, ProviderNormalizer};
pub use request::ProviderRequestPreparer;
