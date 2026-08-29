use super::ObserveContext;
use crate::observe::point::{
    ProviderHttpRequestPrepared, ProviderProtocolRequestPrepared, ProviderStreamOutcomeObserved,
    RequestTranslationFailure, StreamingTranslationFailure,
};

impl ObserveContext {
    pub(crate) fn observe_request_translation_failure(&self, point: RequestTranslationFailure<'_>) {
        self.span.in_scope(|| {
            self.sinks
                .observe_request_translation_failure(self.request_id, &point)
        });
        self.mark_failure_reported();
    }

    pub(crate) fn observe_streaming_translation_failure(
        &self,
        point: StreamingTranslationFailure<'_>,
    ) {
        self.span.in_scope(|| {
            self.sinks
                .observe_streaming_translation_failure(self.request_id, &point)
        });
    }

    pub(crate) fn observe_provider_request_prepared(
        &self,
        event: ProviderProtocolRequestPrepared<'_>,
    ) {
        self.span.in_scope(|| {
            self.sinks
                .observe_provider_request_prepared(self.request_id, &event)
        });
    }

    pub(crate) fn observe_provider_http_request_prepared(
        &self,
        point: ProviderHttpRequestPrepared<'_>,
    ) {
        self.sinks.observe_provider_http_request_prepared(point);
    }

    pub(crate) fn observe_provider_stream_outcome(&self, point: ProviderStreamOutcomeObserved<'_>) {
        self.span.in_scope(|| {
            self.sinks
                .observe_provider_stream_outcome(self.request_id, &point)
        });
    }
}
