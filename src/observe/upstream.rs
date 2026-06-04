use super::ObserveContext;
use crate::error::UpstreamError;
use crate::http_support::UpstreamResponseHead;
use crate::observe::point::{
    UpstreamErrorResponseReceived, UpstreamNonStreamingResponseReceived,
    UpstreamStreamChunkReceived, UpstreamStreamProgress, UpstreamStreamingResponseStarted,
};

impl ObserveContext {
    pub(crate) fn observe_upstream_non_streaming_success(
        &self,
        point: UpstreamNonStreamingResponseReceived<'_>,
    ) {
        self.span
            .in_scope(|| self.sinks.observe_upstream_non_streaming_success(point));
    }

    pub(crate) fn observe_upstream_streaming_success(
        &self,
        point: UpstreamStreamingResponseStarted<'_>,
    ) {
        self.span
            .in_scope(|| self.sinks.observe_upstream_streaming_success(point));
    }

    pub(crate) fn observe_upstream_stream_chunk(&self, point: UpstreamStreamChunkReceived<'_>) {
        self.sinks.observe_upstream_stream_chunk(point);
    }

    pub(crate) fn observe_upstream_stream_wait(&self, point: UpstreamStreamProgress) {
        self.span
            .in_scope(|| self.sinks.observe_upstream_stream_wait(point));
    }

    pub(crate) fn observe_upstream_stream_timeout(&self, point: UpstreamStreamProgress) {
        self.span
            .in_scope(|| self.sinks.observe_upstream_stream_timeout(point));
    }

    pub(crate) fn observe_upstream_body_read_error(
        &self,
        head: &UpstreamResponseHead,
        error: &UpstreamError,
    ) {
        self.span
            .in_scope(|| self.sinks.observe_upstream_body_read_error(head, error));
    }

    pub(crate) fn observe_upstream_error_response(
        &self,
        point: UpstreamErrorResponseReceived<'_>,
        error: &UpstreamError,
    ) {
        self.span
            .in_scope(|| self.sinks.observe_upstream_error_response(point, error));
    }
}
