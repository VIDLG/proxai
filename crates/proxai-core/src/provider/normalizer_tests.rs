use std::sync::{Arc, Mutex};

use serde_json::json;

use crate::observe::{
    Observation, Observer, ProviderObservation, ProviderResponseAdaptation, ProviderResponsePhase,
};
use crate::protocol::ProviderProtocol;
use crate::translation::stream::StreamEvent;

use super::{ProviderCompatibility, ProviderNormalizer};

#[test]
fn structured_normalization_is_required_only_for_supported_compatible_protocols() {
    for protocol in [
        ProviderProtocol::AnthropicMessages,
        ProviderProtocol::OpenaiChatCompletions,
    ] {
        assert!(
            ProviderNormalizer::new(protocol, ProviderCompatibility::Compatible)
                .requires_structured_normalization()
        );
        assert!(
            !ProviderNormalizer::new(protocol, ProviderCompatibility::Strict)
                .requires_structured_normalization()
        );
    }

    assert!(
        !ProviderNormalizer::new(
            ProviderProtocol::OpenaiResponses,
            ProviderCompatibility::Compatible,
        )
        .requires_structured_normalization()
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

    let normalized = ProviderNormalizer::new(
        ProviderProtocol::AnthropicMessages,
        ProviderCompatibility::Strict,
    )
    .normalize_response(payload.clone());

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

    let normalized = ProviderNormalizer::new(
        ProviderProtocol::AnthropicMessages,
        ProviderCompatibility::Compatible,
    )
    .normalize_stream_event(event);

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

    ProviderNormalizer::new(
        ProviderProtocol::OpenaiChatCompletions,
        ProviderCompatibility::Compatible,
    )
    .with_observer(observations)
    .normalize_stream_event(event);

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

#[derive(Clone, Default)]
struct RecordingObserver {
    values: Arc<Mutex<Vec<Observation>>>,
}

impl Observer for RecordingObserver {
    fn observe(&self, observation: &Observation) {
        self.values.lock().unwrap().push(observation.clone());
    }
}
