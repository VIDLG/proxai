use std::sync::{Arc, Mutex};

use super::capture::{CaptureSession, UpstreamResponseCaptureWriter};
use super::diagnostics::DiagnosticsSink;
use super::logging::LoggingSink;
use crate::error::UpstreamError;
use crate::http_support::{ContentType, OutboundResponseHead, UpstreamResponseHead};
use crate::observe::point::{
    InboundRequestPrepared, InboundRequestReceived, OutboundResponseHeadPrepared,
    ProviderHttpRequestPrepared, ProviderProtocolRequestPrepared, ProviderStreamOutcome,
    ProviderStreamOutcomeObserved, ProviderStreamSnapshot, RequestInfoParseFailure,
    UpstreamErrorResponseReceived, UpstreamNonStreamingResponseReceived,
    UpstreamResponseHeadReceived, UpstreamStreamChunkReceived, UpstreamStreamProgress,
    UpstreamStreamingResponseStarted,
};

use crate::request::RequestId;

#[derive(Clone)]
pub(super) struct ObserveSinks {
    pub(super) capture: CaptureSink,
    pub(super) logging: LoggingSink,
    pub(super) diagnostics: DiagnosticsSink,
}

impl ObserveSinks {
    pub(super) fn new(request_id: RequestId, capture: CaptureSession) -> Self {
        Self {
            capture: CaptureSink::new(capture),
            logging: LoggingSink,
            diagnostics: DiagnosticsSink::new(request_id),
        }
    }

    pub(super) fn observe_request_failed(&self, error: &crate::error::Error) {
        self.logging.emit_request_failed(error);
    }

    pub(super) fn observe_inbound_request_received(
        &self,
        request_id: RequestId,
        point: InboundRequestReceived<'_>,
    ) {
        self.logging.emit_inbound_request_received(
            request_id,
            point.method,
            point.uri,
            point.headers,
        );
    }

    pub(super) fn observe_inbound_request_prepared(&self, point: InboundRequestPrepared<'_>) {
        self.capture
            .record_inbound_request(point.method, point.uri, point.headers, point.body);
    }

    pub(super) fn observe_request_info_parse_failure(
        &self,
        request_id: RequestId,
        point: RequestInfoParseFailure<'_>,
    ) {
        let _diagnostic_path = self.diagnostics.record_request_info_parse_failure(
            point.normalized_payload,
            point.request_info_parse_payload,
            point.error,
        );
        self.logging
            .emit_request_info_parse_failure(request_id, point.error);
    }

    pub(super) fn observe_provider_request_prepared(
        &self,
        request_id: RequestId,
        point: &ProviderProtocolRequestPrepared<'_>,
    ) {
        let capture = self.capture.provider_request_enabled();
        self.logging
            .emit_provider_request_prepared(request_id, point, capture);
    }

    pub(super) fn observe_provider_http_request_prepared(
        &self,
        point: ProviderHttpRequestPrepared<'_>,
    ) {
        self.capture.record_provider_request(
            point.method,
            point.url,
            point.headers,
            point.body,
            point.normalized_payload,
        );
    }

    pub(super) fn observe_provider_stream_outcome(
        &self,
        point: &ProviderStreamOutcomeObserved<'_>,
    ) {
        let diagnostic_path = match (&point.snapshot, &point.outcome) {
            (
                ProviderStreamSnapshot::OpenaiResponses(snapshot),
                ProviderStreamOutcome::UnfinishedTool(_),
            ) => self
                .diagnostics
                .record_openai_responses_unfinished_tool_stream(snapshot),
            _ => None,
        };
        self.logging
            .emit_provider_stream_outcome(point, diagnostic_path.as_deref());
    }

    pub(super) fn observe_upstream_response_head_received(
        &self,
        point: UpstreamResponseHeadReceived<'_>,
    ) {
        self.capture.record_upstream_response_headers(point.head);
        self.logging.emit_head_info(point.head);
    }

    pub(super) fn observe_upstream_non_streaming_success(
        &self,
        point: UpstreamNonStreamingResponseReceived<'_>,
    ) {
        self.observe_upstream_response_head_received(UpstreamResponseHeadReceived {
            head: point.head,
        });
        self.capture
            .record_upstream_response_body(point.head.content_type(), point.body);
    }

    pub(super) fn observe_upstream_streaming_success(
        &self,
        point: UpstreamStreamingResponseStarted<'_>,
    ) {
        self.observe_upstream_response_head_received(UpstreamResponseHeadReceived {
            head: point.head,
        });
        self.capture
            .start_upstream_stream(point.head.content_type().as_ref());
    }

    pub(super) fn observe_upstream_stream_chunk(&self, point: UpstreamStreamChunkReceived<'_>) {
        self.capture.record_upstream_stream_chunk(point.chunk);
    }

    pub(super) fn observe_upstream_stream_wait(&self, point: UpstreamStreamProgress) {
        self.logging.emit_stream_wait(point);
    }

    pub(super) fn observe_upstream_stream_timeout(&self, point: UpstreamStreamProgress) {
        self.logging.emit_stream_timeout(point);
    }

    pub(super) fn observe_upstream_body_read_error(
        &self,
        head: &UpstreamResponseHead,
        error: &UpstreamError,
    ) {
        self.capture.record_upstream_response_headers(head);
        self.log_upstream_error(error);
    }

    pub(super) fn observe_upstream_error_response(
        &self,
        point: UpstreamErrorResponseReceived<'_>,
        error: &UpstreamError,
    ) {
        self.capture
            .record_upstream_response(point.head, point.body);
        self.log_upstream_error(error);
    }

    pub(super) fn observe_outbound_response_head_prepared(
        &self,
        point: OutboundResponseHeadPrepared<'_>,
    ) {
        self.capture.record_outbound_response_head(point.head);
    }

    fn log_upstream_error(&self, error: &UpstreamError) {
        let head = match error {
            UpstreamError::ErrorStatus { head, .. }
            | UpstreamError::ResponseBodyRead { head, .. } => head,
            UpstreamError::RequestSend(_) => return,
        };

        self.logging.emit_head_error(head, error);
    }
}

#[derive(Clone)]
pub(super) struct CaptureSink {
    session: CaptureSession,
    upstream_stream_writer: Arc<Mutex<Option<UpstreamResponseCaptureWriter>>>,
}

impl CaptureSink {
    fn new(session: CaptureSession) -> Self {
        Self {
            session,
            upstream_stream_writer: Arc::new(Mutex::new(None)),
        }
    }

    pub(super) fn provider_request_enabled(&self) -> bool {
        self.session.provider_request_enabled()
    }

    pub(super) fn record_inbound_request(
        &self,
        method: &axum::http::Method,
        uri: &axum::http::Uri,
        headers: &axum::http::HeaderMap,
        body: &axum::body::Bytes,
    ) {
        self.session
            .capture_inbound_request(method, uri, headers, body);
    }

    pub(super) fn record_provider_request(
        &self,
        method: &axum::http::Method,
        url: &str,
        headers: &axum::http::HeaderMap,
        body: &[u8],
        normalized_payload: Option<&serde_json::Value>,
    ) {
        self.session
            .capture_provider_request(method, url, headers, body, normalized_payload);
    }

    pub(super) fn record_upstream_response_headers(&self, head: &UpstreamResponseHead) {
        self.session.capture_upstream_response_headers(head);
    }

    pub(super) fn record_upstream_response_body(
        &self,
        content_type: Option<ContentType>,
        body: &[u8],
    ) {
        self.session
            .capture_upstream_response_body(content_type, body);
    }

    pub(super) fn record_upstream_response(&self, head: &UpstreamResponseHead, body: &[u8]) {
        self.session.capture_upstream_response(head, body);
    }

    pub(super) fn start_upstream_stream(&self, content_type: Option<&ContentType>) {
        let writer = self.session.create_upstream_response_writer(content_type);
        *self
            .upstream_stream_writer
            .lock()
            .expect("stream capture writer lock poisoned") = writer;
    }

    pub(super) fn record_upstream_stream_chunk(&self, chunk: &[u8]) {
        if let Some(writer) = self
            .upstream_stream_writer
            .lock()
            .expect("stream capture writer lock poisoned")
            .as_mut()
        {
            writer.write_chunk(chunk);
        }
    }

    pub(super) fn record_outbound_response_head(&self, head: &OutboundResponseHead) {
        self.session.capture_outbound_response_headers(
            head.status(),
            head.content_type().as_ref().map(AsRef::as_ref),
            head.headers(),
        );
    }
}
