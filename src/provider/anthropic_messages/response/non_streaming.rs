use axum::body::{Body, Bytes};
use axum::http::Response;

use crate::http_model::UpstreamResponseHead;
use crate::provider::ProviderNonStreamingResponseContext;
use crate::upstream::forward_non_streaming_response;

use super::normalize;

pub(crate) async fn handle_non_streaming_response(
    context: ProviderNonStreamingResponseContext<'_>,
    head: UpstreamResponseHead,
    body: Bytes,
) -> Response<Body> {
    let ProviderNonStreamingResponseContext {
        capture,
        span,
        compatibility,
    } = context;
    forward_non_streaming_response(capture, span, head, body, |body| {
        if matches!(
            compatibility,
            crate::config::ProviderCompatibility::AnthropicCompatible
        ) {
            normalize::normalize_message_body_bytes(&body).unwrap_or(body)
        } else {
            body
        }
    })
    .await
}
