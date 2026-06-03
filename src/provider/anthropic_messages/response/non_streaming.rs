use axum::body::{Body, Bytes};
use axum::http::Response;

use crate::config::ProviderCompatibility;
use crate::http_support::UpstreamResponseHead;
use crate::observe::ObserveContext;
use crate::upstream::forward_non_streaming_response;

use super::normalize;

pub(crate) fn handle_non_streaming_response(
    obs: &ObserveContext,
    compatibility: ProviderCompatibility,
    head: UpstreamResponseHead,
    body: Bytes,
) -> Response<Body> {
    forward_non_streaming_response(obs, head, body, |body| {
        if matches!(
            compatibility,
            crate::config::ProviderCompatibility::AnthropicCompatible
        ) {
            normalize::normalize_message_body_bytes(&body).unwrap_or(body)
        } else {
            body
        }
    })
}
