use std::sync::{Arc, Mutex};

use serde_json::json;

use crate::observe::{
    NoopObserver, Observation, Observer, ProviderObservation, ProviderResponseAdaptation,
    ProviderResponsePhase,
};
use crate::protocol::ProviderProtocol;
use crate::translation::stream::StreamEvent;

use super::{
    ProviderBehavior, ProviderCompatibility, normalize_provider_response,
    normalize_provider_stream_event,
};

#[test]
fn identity_normalization_is_required_only_for_measured_compatible_protocols() {
    for protocol in [
        ProviderProtocol::AnthropicMessages,
        ProviderProtocol::OpenaiChatCompletions,
    ] {
        assert!(
            ProviderBehavior::new(protocol, ProviderCompatibility::Compatible)
                .requires_identity_normalization()
        );
        assert!(
            !ProviderBehavior::new(protocol, ProviderCompatibility::Strict)
                .requires_identity_normalization()
        );
    }

    assert!(
        !ProviderBehavior::new(
            ProviderProtocol::OpenaiResponses,
            ProviderCompatibility::Compatible,
        )
        .requires_identity_normalization()
    );
}

#[test]
fn strict_mode_preserves_provider_payloads() {
    let payload = json!({
        "id": "msg_1",
        "role": "assistant",
        "model": "claude",
        "content": [],
        "usage": {"input_tokens": 1, "output_tokens": 1}
    });

    let normalized = normalize_provider_response(
        ProviderBehavior::new(
            ProviderProtocol::AnthropicMessages,
            ProviderCompatibility::Strict,
        ),
        payload.clone(),
        &NoopObserver,
    );

    assert_eq!(normalized, payload);
}

#[test]
fn compatible_mode_normalizes_structured_stream_events() {
    let event = StreamEvent::message(json!({
        "type": "message_delta",
        "delta": {"stop_reason": "end_turn"},
        "usage": {"output_tokens": 1}
    }))
    .unwrap();

    let normalized = normalize_provider_stream_event(
        ProviderBehavior::new(
            ProviderProtocol::AnthropicMessages,
            ProviderCompatibility::Compatible,
        ),
        event,
        &NoopObserver,
    );

    assert_eq!(normalized.event_type, "message_delta");
    assert_eq!(normalized.data["delta"]["stop_sequence"], json!(null));
    assert_eq!(normalized.data["usage"]["input_tokens"], json!(null));
}

#[test]
fn compatible_mode_emits_typed_provider_observation() {
    let observations = RecordingObserver::default();
    let recorded = observations.values.clone();
    let event = StreamEvent::message(json!({
        "choices": [{"index": 0, "delta": {"content": "hello"}}]
    }))
    .unwrap();

    normalize_provider_stream_event(
        ProviderBehavior::new(
            ProviderProtocol::OpenaiChatCompletions,
            ProviderCompatibility::Compatible,
        ),
        event,
        &observations,
    );

    assert!(matches!(
        recorded.lock().unwrap().as_slice(),
        [Observation::Provider(
            ProviderObservation::ResponseAdapted {
                phase: ProviderResponsePhase::Streaming,
                adaptation: ProviderResponseAdaptation::OpenaiChatCompletionsStreamEvent,
                ..
            }
        )]
    ));
}

#[test]
fn compatible_responses_usage_repair_emits_typed_provider_observation() {
    let observations = RecordingObserver::default();
    let recorded = observations.values.clone();
    let event = StreamEvent::message(json!({
        "type": "response.completed",
        "response": {
            "usage": {
                "input_tokens_details": {"cached_tokens": 1}
            }
        }
    }))
    .unwrap();

    let normalized = normalize_provider_stream_event(
        ProviderBehavior::new(
            ProviderProtocol::OpenaiResponses,
            ProviderCompatibility::Compatible,
        ),
        event,
        &observations,
    );

    assert_eq!(
        normalized.data["response"]["usage"]["input_tokens_details"]["cache_write_tokens"],
        0
    );
    assert!(matches!(
        recorded.lock().unwrap().as_slice(),
        [Observation::Provider(
            ProviderObservation::ResponseAdapted {
                phase: ProviderResponsePhase::Streaming,
                adaptation: ProviderResponseAdaptation::OpenaiResponsesUsageShape,
                ..
            }
        )]
    ));
}

#[derive(Clone, Default)]
struct RecordingObserver {
    values: Arc<Mutex<Vec<Observation>>>,
}

impl Observer for RecordingObserver {
    fn observe(&self, observation: &Observation) {
        self.values.lock().unwrap().push(observation.clone());
    }
}
