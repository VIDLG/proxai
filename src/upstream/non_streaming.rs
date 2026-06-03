use axum::body::{Body, Bytes};
use axum::http::Response;

use crate::capture::CaptureSession;
use crate::http_model::UpstreamResponseHead;
use crate::http_utils::{filter_forwardable_headers, response_with_headers};
use crate::logging;

pub(crate) async fn forward_non_streaming_response(
    capture: &CaptureSession,
    span: &tracing::Span,
    head: UpstreamResponseHead,
    body: Bytes,
    transform_body: impl FnOnce(Bytes) -> Bytes,
) -> Response<Body> {
    capture.capture_upstream_response_headers(&head).await;

    let outbound_headers = filter_forwardable_headers(&head.headers);
    capture
        .capture_outbound_response_headers(
            head.status,
            head.content_type().as_ref().map(AsRef::as_ref),
            &outbound_headers,
        )
        .await;

    capture
        .capture_upstream_response_body(head.content_type().as_ref(), &body)
        .await;

    span.in_scope(|| logging::UpstreamLogRecord::HeadInfo { head: &head }.emit());

    response_with_headers(
        head.status,
        outbound_headers,
        Body::from(transform_body(body)),
    )
}
