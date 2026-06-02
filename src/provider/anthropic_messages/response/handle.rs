use axum::body::Body;
use axum::http::{HeaderMap, Response};

use crate::config::ProviderCompatibility;
use crate::error::Result;
use crate::logging;
use crate::provider::{filter_forwardable_headers, OutboundResponseContext};
use crate::upstream::UpstreamResponseHead;

use super::normalize;
use super::snapshot::AnthropicUpstreamResponseSnapshot;
use super::stream::handle_streaming;
use super::tracker::AnthropicResponseTracker;

/// Handle a successful upstream response where the provider protocol is `AnthropicMessages`.
pub(crate) async fn handle_success_response(
    ctx: OutboundResponseContext<'_>,
    upstream_response: reqwest::Response,
) -> Result<Response<Body>> {
    let upstream_headers = upstream_response.headers().clone();

    let head = UpstreamResponseHead::from_headers(
        upstream_response.status(),
        &upstream_headers,
        ctx.started.elapsed(),
    );
    ctx.capture
        .capture_upstream_response_headers(&head, &upstream_headers)
        .await?;

    ctx.span
        .in_scope(|| logging::UpstreamLogRecord::HeadInfo { head: &head }.emit());

    let outbound_headers = filter_forwardable_headers(&upstream_headers);

    ctx.capture
        .capture_outbound_response_headers(
            head.status,
            head.content_type
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            &outbound_headers,
        )
        .await?;

    let is_sse = head.is_sse();
    if is_sse {
        handle_streaming(
            ctx,
            upstream_response,
            &upstream_headers,
            &head,
            outbound_headers,
        )
        .await
    } else {
        handle_non_streaming(ctx, upstream_response, &head, outbound_headers).await
    }
}

async fn handle_non_streaming(
    ctx: OutboundResponseContext<'_>,
    upstream_response: reqwest::Response,
    upstream_head: &UpstreamResponseHead,
    outbound_headers: HeaderMap,
) -> Result<Response<Body>> {
    let body_bytes = upstream_response
        .bytes()
        .await
        .map_err(|e| crate::error::InternalError::Io(std::io::Error::other(e.to_string())))?;

    let mut tracker = AnthropicResponseTracker::from_headers(&HeaderMap::new());
    tracker.scan_bytes(&body_bytes);
    tracker.finish();
    let snapshot = AnthropicUpstreamResponseSnapshot::non_streaming(
        upstream_head,
        body_bytes.len(),
        ctx.started.elapsed(),
        tracker.state.clone(),
    );
    ctx.span.in_scope(|| {
        logging::AnthropicLogRecord::Completed {
            snapshot: &snapshot,
        }
        .emit()
    });

    ctx.capture
        .capture_upstream_response_body(upstream_head.content_type.as_ref(), &body_bytes)
        .await?;

    let outbound_body = if should_normalize_provider_response(ctx.provider_compatibility) {
        normalize::normalize_body_bytes(&body_bytes).unwrap_or(body_bytes)
    } else {
        body_bytes
    };

    let mut response = Response::new(Body::from(outbound_body));
    *response.status_mut() = upstream_head.status;
    *response.headers_mut() = outbound_headers;
    Ok(response)
}

fn should_normalize_provider_response(compatibility: ProviderCompatibility) -> bool {
    matches!(compatibility, ProviderCompatibility::AnthropicCompatible)
}
