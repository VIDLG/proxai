use super::ObserveContext;
use crate::observe::logging;
use crate::observe::point::{
    InboundRequestPrepared, InboundRequestReceived, RequestFailed, RequestInfoParseFailure,
};

impl ObserveContext {
    pub(crate) fn observe_request_failed(&self, point: RequestFailed<'_>) {
        self.span
            .in_scope(|| logging::emit_request_failed(point.error));
    }

    pub(crate) fn observe_inbound_request_received(&self, point: InboundRequestReceived<'_>) {
        self.span.in_scope(|| {
            logging::emit_inbound_request_received(
                self.request_id,
                point.method,
                point.uri,
                point.headers,
            )
        });
    }

    pub(crate) fn observe_inbound_request_prepared(&self, event: InboundRequestPrepared<'_>) {
        self.capture
            .capture_inbound_request(event.method, event.uri, event.headers, event.body);
    }

    pub(crate) fn observe_request_info_parse_failure(&self, point: RequestInfoParseFailure<'_>) {
        let _diagnostic_path = self.diagnostics.record_request_info_parse_failure(
            point.normalized_payload,
            point.request_info_parse_payload,
            point.error,
        );
        self.span
            .in_scope(|| logging::emit_request_info_parse_failure(self.request_id, point.error));
    }
}
