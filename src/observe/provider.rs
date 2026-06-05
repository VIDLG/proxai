use super::ObserveContext;
use crate::observe::point::{
    ProviderHttpRequestPrepared, ProviderProtocolRequestPrepared, ProviderStreamOutcomeObserved,
};

impl ObserveContext {
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
        self.span
            .in_scope(|| self.sinks.observe_provider_stream_outcome(&point));
    }
}
