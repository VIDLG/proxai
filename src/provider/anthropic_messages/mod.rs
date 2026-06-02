//! Pass-through response handler for the Anthropic Messages API.
//!
//! When the inbound protocol is `AnthropicMessages` and the provider protocol is also
//! `AnthropicMessages`, no translation is needed — the upstream response is forwarded
//! verbatim to the client.

pub(crate) mod request;
mod response;

pub(crate) use self::response::normalize;

pub(crate) use self::response::{
    handle_success_response, AnthropicResponseOutputKind, AnthropicUpstreamResponseSnapshot,
};
