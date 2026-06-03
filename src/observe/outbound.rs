use super::ObserveContext;
use crate::observe::point::OutboundResponseHeadPrepared;

impl ObserveContext {
    pub(crate) fn observe_outbound_response_head_prepared(
        &self,
        point: OutboundResponseHeadPrepared<'_>,
    ) {
        let outbound = point.head;
        self.capture.capture_outbound_response_headers(
            outbound.status(),
            outbound.content_type().as_ref().map(AsRef::as_ref),
            outbound.headers(),
        );
    }
}
