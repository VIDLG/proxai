use super::ObserveContext;
use crate::observe::logging;
use crate::observe::point::{
    ProviderHttpRequestPrepared, ProviderRequestPrepared, ProviderStreamChunkObserved,
    ProviderStreamOutcome, ProviderStreamOutcomeObserved, ProviderStreamSnapshot,
};
use crate::protocol::ProviderProtocol;

impl ObserveContext {
    pub(crate) fn observe_provider_request_prepared(&self, event: ProviderRequestPrepared<'_>) {
        let capture = self.capture.provider_request_enabled();
        self.span
            .in_scope(|| logging::emit_provider_request_prepared(self.request_id, &event, capture));
    }

    pub(crate) fn observe_provider_http_request_prepared(
        &self,
        point: ProviderHttpRequestPrepared<'_>,
    ) {
        self.capture.capture_provider_request(
            point.method,
            point.url,
            point.headers,
            point.body,
            point.normalized_payload,
        );
    }

    pub(crate) fn observe_provider_stream_chunk(&self, point: ProviderStreamChunkObserved<'_>) {
        if matches!(point.provider_protocol, ProviderProtocol::OpenaiResponses) {
            self.diagnostics
                .observe_openai_responses_stream_chunk(point.chunk);
        }
    }

    pub(crate) fn observe_provider_stream_outcome(&self, point: ProviderStreamOutcomeObserved<'_>) {
        let diagnostic_path = match (&point.snapshot, &point.outcome) {
            (
                ProviderStreamSnapshot::OpenaiResponses(snapshot),
                ProviderStreamOutcome::UnfinishedTool(_),
            ) => self
                .diagnostics
                .record_openai_responses_unfinished_tool_stream(snapshot),
            _ => None,
        };
        self.span
            .in_scope(|| logging::emit_provider_stream_outcome(&point, diagnostic_path.as_deref()));
    }
}
