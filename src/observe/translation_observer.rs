use crate::translation::{TranslationObservation, TranslationObserver};

use super::ObserveContext;

impl TranslationObserver for ObserveContext {
    fn observe(&self, observation: &TranslationObservation) {
        self.span.in_scope(|| {
            tracing::trace!(
                request_protocol = %observation.route.request_protocol,
                provider_protocol = %observation.route.provider_protocol,
                phase = ?observation.phase,
                kind = ?observation.kind,
                subject = observation.subject,
                detail = observation.detail,
                "protocol translation observation"
            );
        });
    }
}
