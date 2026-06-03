use axum::body::Bytes;

use crate::capture::CaptureSession;
use crate::error::{UpstreamError, UpstreamResponseError};
use crate::http_model::UpstreamResponseHead;
use crate::logging;

pub(crate) async fn classify_error_response(
    capture: &CaptureSession,
    span: &tracing::Span,
    upstream_head: UpstreamResponseHead,
    body: Bytes,
) -> UpstreamError {
    capture
        .capture_upstream_response(&upstream_head, &body)
        .await;

    let parsed = UpstreamResponseError::parse_body(&body);
    let error = UpstreamError::ErrorStatus {
        head: upstream_head,
        body,
        parsed,
    };
    log_upstream_error(span, &error);
    error
}

pub(crate) async fn log_upstream_body_read_error(
    capture: &CaptureSession,
    span: &tracing::Span,
    upstream_head: &UpstreamResponseHead,
    error: &UpstreamError,
) {
    capture
        .capture_upstream_response_headers(upstream_head)
        .await;
    span.in_scope(|| {
        logging::UpstreamLogRecord::HeadError {
            head: upstream_head,
            error,
        }
        .emit()
    });
}

pub(crate) fn log_upstream_error(span: &tracing::Span, error: &UpstreamError) {
    let head = match error {
        UpstreamError::ErrorStatus { head, .. } | UpstreamError::ResponseBodyRead { head, .. } => {
            head
        }
        UpstreamError::RequestSend(_) => return,
    };

    span.in_scope(|| logging::UpstreamLogRecord::HeadError { head, error }.emit());
}
