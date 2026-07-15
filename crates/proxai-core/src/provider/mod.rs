//! Carrier-independent provider request preparation, response/error normalization,
//! and compatibility handling.
//!
//! This module operates on structured JSON values and stream events only. HTTP
//! transport, authentication, byte framing, timeouts, and concrete observation
//! sinks belong to downstream composition.

mod error;
mod normalizer;
mod request;
mod response;

pub use error::{ProviderError, normalize_provider_error};
pub use normalizer::{ProviderCompatibility, ProviderNormalizer};
pub use request::ProviderRequestPreparer;
