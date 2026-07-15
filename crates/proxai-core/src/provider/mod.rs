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

use getset::CopyGetters;

use crate::protocol::ProviderProtocol;

pub use error::{ProviderError, normalize_provider_error};
pub use normalizer::{
    ProviderCompatibility, normalize_provider_response, normalize_provider_stream_event,
    requires_structured_normalization,
};
pub use request::prepare_provider_request;

/// Carrier-independent properties of a configured provider.
///
/// Transport details such as the base URL, credentials, headers, and timeouts
/// remain in downstream composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, CopyGetters)]
pub struct ProviderBehavior {
    #[getset(get_copy = "pub")]
    protocol: ProviderProtocol,
    #[getset(get_copy = "pub")]
    compatibility: ProviderCompatibility,
}

impl ProviderBehavior {
    pub fn new(protocol: ProviderProtocol, compatibility: ProviderCompatibility) -> Self {
        Self {
            protocol,
            compatibility,
        }
    }
}
