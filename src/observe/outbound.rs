use super::ObserveContext;
use crate::observe::point::OutboundResponseHeadPrepared;

impl ObserveContext {
    pub(crate) fn observe_outbound_response_head_prepared(
        &self,
        point: OutboundResponseHeadPrepared<'_>,
    ) {
        self.sinks.observe_outbound_response_head_prepared(point);
    }
}
