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
pub(crate) use normalizer::normalize_provider_response;
pub use normalizer::{ProviderCompatibility, normalize_provider_stream_event};
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

    /// Returns whether structured provider responses may receive compatibility repairs.
    pub(crate) fn uses_compatibility_repairs(self) -> bool {
        self.compatibility == ProviderCompatibility::Compatible
    }

    /// Returns whether identity forwarding must decode structured responses so
    /// measured provider compatibility gaps can be repaired.
    pub(crate) fn requires_identity_normalization(self) -> bool {
        self.uses_compatibility_repairs()
            && matches!(
                self.protocol,
                ProviderProtocol::AnthropicMessages | ProviderProtocol::OpenaiChatCompletions
            )
    }
}
