use std::sync::Arc;

use serde_json::Value;

use crate::observe::{NoopObserver, Observer, ProviderObservation, ProviderRequestAdaptation};
use crate::protocol::ProviderProtocol;

mod openai_responses;

/// Carrier-independent provider request payload preparation.
///
/// This stage rewrites the routed upstream model and applies protocol-specific
/// compatibility adaptations. Serialization, request paths, authentication,
/// and HTTP transport remain the downstream carrier's responsibility.
#[derive(Clone)]
pub struct ProviderRequestPreparer {
    protocol: ProviderProtocol,
    observer: Arc<dyn Observer>,
}

impl ProviderRequestPreparer {
    pub fn new(protocol: ProviderProtocol) -> Self {
        Self {
            protocol,
            observer: Arc::new(NoopObserver),
        }
    }

    pub fn with_observer(mut self, observer: impl Observer + 'static) -> Self {
        self.observer = Arc::new(observer);
        self
    }

    pub fn prepare(&self, mut payload: Value, upstream_model: &str) -> Value {
        if let Some(model) = payload.get_mut("model") {
            *model = Value::String(upstream_model.to_string());
        }

        if self.protocol != ProviderProtocol::OpenaiResponses {
            return payload;
        }

        let (payload, sanitized) = openai_responses::sanitize_provider_payload(payload);
        if sanitized.status_removed > 0 || sanitized.reasoning_content_removed > 0 {
            self.observer.observe(
                &ProviderObservation::RequestAdapted {
                    protocol: self.protocol,
                    adaptation: ProviderRequestAdaptation::OpenaiResponsesOutputFieldsRemoved {
                        status_removed: sanitized.status_removed,
                        reasoning_content_removed: sanitized.reasoning_content_removed,
                    },
                }
                .into(),
            );
        }
        payload
    }
}

#[cfg(test)]
mod tests;
