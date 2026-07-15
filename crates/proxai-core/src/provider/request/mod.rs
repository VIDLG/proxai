use serde_json::Value;

use crate::observe::{Observer, ProviderObservation, ProviderRequestAdaptation};
use crate::protocol::ProviderProtocol;

mod openai_responses;

/// Prepare a carrier-independent provider request payload.
///
/// This rewrites the routed upstream model and applies protocol-specific
/// compatibility adaptations. Serialization, request paths, authentication,
/// and HTTP transport remain the downstream carrier's responsibility.
pub fn prepare_provider_request(
    protocol: ProviderProtocol,
    mut payload: Value,
    upstream_model: &str,
    observer: &dyn Observer,
) -> Value {
    if let Some(model) = payload.get_mut("model") {
        *model = Value::String(upstream_model.to_string());
    }

    if protocol != ProviderProtocol::OpenaiResponses {
        return payload;
    }

    let (payload, sanitized) = openai_responses::sanitize_provider_payload(payload);
    if sanitized.status_removed > 0 || sanitized.reasoning_content_removed > 0 {
        observer.observe(
            &ProviderObservation::RequestAdapted {
                protocol,
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

#[cfg(test)]
mod tests;
