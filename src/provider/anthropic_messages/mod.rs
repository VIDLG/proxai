//! Pass-through response handler for the Anthropic Messages API.
//!
//! When the inbound protocol is `AnthropicMessages` and the provider protocol is also
//! `AnthropicMessages`, no translation is needed — the upstream response is forwarded
//! verbatim to the client.

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
    handle_non_streaming_response, handle_streaming_response, AnthropicResponseOutputKind,
    AnthropicUpstreamResponseSnapshot,
};
