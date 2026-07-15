use std::sync::{Arc, Mutex};

use serde_json::json;

use crate::observe::{Observation, Observer, ProviderObservation, ProviderRequestAdaptation};
use crate::protocol::ProviderProtocol;

use super::ProviderRequestPreparer;

#[test]
fn rewrites_existing_model_for_every_provider_protocol() {
    for protocol in [
        ProviderProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiChatCompletions,
        ProviderProtocol::AnthropicMessages,
    ] {
        let prepared = ProviderRequestPreparer::new(protocol)
            .prepare(json!({"model": "client-model"}), "upstream-model");

        assert_eq!(prepared["model"], "upstream-model");
    }
}

#[test]
fn does_not_invent_a_missing_model_field() {
    let payload = json!({"input": "hello"});

    let prepared = ProviderRequestPreparer::new(ProviderProtocol::OpenaiResponses)
        .prepare(payload.clone(), "upstream-model");

    assert_eq!(prepared, payload);
}

#[test]
fn responses_preparation_removes_only_output_fields_invalid_as_input() {
    let payload = json!({
        "model": "client-model",
        "status": "completed",
        "input": [
            {
                "type": "message",
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": "previous answer"}]
            },
            {
                "type": "reasoning",
                "id": "rs_123",
                "status": "completed",
                "content": [{"type": "reasoning_text", "text": "reasoning"}],
                "summary": []
            },
            {
                "type": "function_call_output",
                "call_id": "call_123",
                "status": "failed",
                "output": "tool failed"
            }
        ]
    });

    let prepared = ProviderRequestPreparer::new(ProviderProtocol::OpenaiResponses)
        .prepare(payload, "upstream-model");

    assert_eq!(prepared["model"], "upstream-model");
    assert_eq!(prepared["status"], "completed");
    assert!(prepared["input"][0].get("status").is_none());
    assert!(prepared["input"][1].get("status").is_none());
    assert!(prepared["input"][1].get("content").is_none());
    assert_eq!(prepared["input"][1]["summary"], json!([]));
    assert_eq!(prepared["input"][2]["status"], "failed");
}

#[test]
fn responses_preparation_emits_typed_request_adaptation() {
    let observer = RecordingObserver::default();
    let recorded = observer.values.clone();
    let payload = json!({
        "input": [
            {"type": "message", "status": "completed"},
            {"type": "reasoning", "status": "completed", "content": []}
        ]
    });

    ProviderRequestPreparer::new(ProviderProtocol::OpenaiResponses)
        .with_observer(observer)
        .prepare(payload, "upstream-model");

    assert!(matches!(
        recorded.lock().unwrap().as_slice(),
        [Observation::Provider(ProviderObservation::RequestAdapted {
            protocol: ProviderProtocol::OpenaiResponses,
            adaptation: ProviderRequestAdaptation::OpenaiResponsesOutputFieldsRemoved {
                status_removed: 2,
                reasoning_content_removed: 1,
            },
        })]
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
