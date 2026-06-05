//! Pass-through response handler for the Anthropic Messages API.
//!
//! When the inbound and provider protocols are both `AnthropicMessages`, the
//! upstream response is forwarded verbatim to the client.

use axum::http::{HeaderMap, HeaderValue};

pub(crate) mod request;
mod response;

pub(crate) use self::response::normalize;

pub(crate) fn apply_request_auth_headers(headers: &mut HeaderMap, api_key: &str) {
    headers.remove(http::header::AUTHORIZATION);
    if let Ok(value) = HeaderValue::from_str(api_key.trim()) {
        headers.insert("x-api-key", value);
    }
}

pub(crate) use self::response::{
    AnthropicResponseOutputKind, AnthropicUpstreamResponseSnapshot, handle_non_streaming_response,
    handle_streaming_response,
};
