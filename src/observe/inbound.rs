use super::ObserveContext;
use crate::observe::point::{
    InboundRequestPrepared, InboundRequestReceived, RequestFailed, RequestInfoParseFailure,
};

impl ObserveContext {
    pub(crate) fn observe_request_failed(&self, point: RequestFailed<'_>) {
        self.span
            .in_scope(|| self.sinks.observe_request_failed(point.error));
    }

    pub(crate) fn observe_inbound_request_received(&self, point: InboundRequestReceived<'_>) {
        self.span.in_scope(|| {
            self.sinks
                .observe_inbound_request_received(self.request_id, point)
        });
    }

    pub(crate) fn observe_inbound_request_prepared(&self, event: InboundRequestPrepared<'_>) {
        self.sinks.observe_inbound_request_prepared(event);
    }

    pub(crate) fn observe_request_info_parse_failure(&self, point: RequestInfoParseFailure<'_>) {
        self.span.in_scope(|| {
            self.sinks
                .observe_request_info_parse_failure(self.request_id, point)
        });
    }
}
