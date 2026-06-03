use super::ObserveContext;
use crate::error::UpstreamError;
use crate::http_support::UpstreamResponseHead;
use crate::observe::logging;
use crate::observe::point::{
    UpstreamErrorResponseReceived, UpstreamNonStreamingResponseReceived,
    UpstreamResponseHeadReceived, UpstreamStreamChunkReceived, UpstreamStreamProgress,
    UpstreamStreamingResponseStarted,
};

impl ObserveContext {
    pub(crate) fn observe_upstream_response_head_received(
        &self,
        point: UpstreamResponseHeadReceived<'_>,
    ) {
        self.capture.capture_upstream_response_headers(point.head);
        self.record_upstream_head_info(point.head);
    }

    pub(crate) fn observe_upstream_non_streaming_success(
        &self,
        point: UpstreamNonStreamingResponseReceived<'_>,
    ) {
        self.observe_upstream_response_head_received(UpstreamResponseHeadReceived {
            head: point.head,
        });
        self.capture
            .capture_upstream_response_body(point.head.content_type(), point.body);
    }

    pub(crate) fn observe_upstream_streaming_success(
        &self,
        point: UpstreamStreamingResponseStarted<'_>,
    ) {
        self.observe_upstream_response_head_received(UpstreamResponseHeadReceived {
            head: point.head,
        });
        let writer = self
            .capture
            .create_upstream_response_writer(point.head.content_type().as_ref());
        *self
            .stream_capture_writer
            .lock()
            .expect("stream capture writer lock poisoned") = writer;
    }

    pub(crate) fn observe_upstream_stream_chunk(&self, point: UpstreamStreamChunkReceived<'_>) {
        if let Some(writer) = self
            .stream_capture_writer
            .lock()
            .expect("stream capture writer lock poisoned")
            .as_mut()
        {
            writer.write_chunk(point.chunk);
        }
    }

    pub(crate) fn observe_upstream_stream_wait(&self, point: UpstreamStreamProgress) {
        self.span.in_scope(|| logging::emit_stream_wait(point));
    }

    pub(crate) fn observe_upstream_stream_timeout(&self, point: UpstreamStreamProgress) {
        self.span.in_scope(|| logging::emit_stream_timeout(point));
    }

    pub(crate) fn observe_upstream_body_read_error(
        &self,
        head: &UpstreamResponseHead,
        error: &UpstreamError,
    ) {
        self.capture.capture_upstream_response_headers(head);
        self.log_upstream_error(error);
    }

    pub(crate) fn observe_upstream_error_response(
        &self,
        point: UpstreamErrorResponseReceived<'_>,
        error: &UpstreamError,
    ) {
        self.capture
            .capture_upstream_response(point.head, point.body);
        self.log_upstream_error(error);
    }

    fn record_upstream_head_info(&self, head: &UpstreamResponseHead) {
        self.span.in_scope(|| logging::emit_head_info(head));
    }

    fn log_upstream_error(&self, error: &UpstreamError) {
        let head = match error {
            UpstreamError::ErrorStatus { head, .. }
            | UpstreamError::ResponseBodyRead { head, .. } => head,
            UpstreamError::RequestSend(_) => return,
        };

        self.span.in_scope(|| logging::emit_head_error(head, error));
    }
}
