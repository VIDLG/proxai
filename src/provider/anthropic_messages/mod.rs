//! Pass-through response handler for the Anthropic Messages API.
//!
//! When the inbound and provider protocols are both `AnthropicMessages`, the
//! upstream response is forwarded verbatim to the client.

pub(crate) mod request;
mod response;

pub(crate) use self::response::{
    AnthropicResponseOutputKind, AnthropicUpstreamResponseSnapshot, handle_streaming_response,
    normalize_message_body_bytes,
};
