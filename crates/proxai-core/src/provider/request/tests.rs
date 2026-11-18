use std::sync::{Arc, Mutex};

use serde_json::json;

use crate::observe::{
    NoopObserver, Observation, Observer, ProviderObservation, ProviderRequestAdaptation,
};
use crate::protocol::ProviderProtocol;

use super::prepare_provider_request;

#[test]
fn rewrites_existing_model_for_every_provider_protocol() {
    for protocol in [
        ProviderProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiChatCompletions,
        ProviderProtocol::AnthropicMessages,
    ] {
        let prepared = prepare_provider_request(
            protocol,
            json!({"model": "client-model"}),
            "upstream-model",
            &NoopObserver,
        );

        assert_eq!(prepared["model"], "upstream-model");
    }
}

#[test]
fn does_not_invent_a_missing_model_field() {
    let payload = json!({"input": "hello"});

    let prepared = prepare_provider_request(
        ProviderProtocol::OpenaiResponses,
        payload.clone(),
        "upstream-model",
        &NoopObserver,
    );

    assert_eq!(prepared, payload);
}

#[test]
fn responses_preparation_preserves_historical_reasoning_while_stripping_status() {
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
                "content": [
                    {"type": "reasoning_text", "text": "先考虑 🦀\n第二行"},
                    {"type": "reasoning_text", "text": "café e\u{301}"}
                ],
                "summary": [{"type": "summary_text", "text": "previous reasoning summary"}],
                "encrypted_content": "opaque-reasoning"
            },
            {
                "type": "function_call_output",
                "call_id": "call_123",
                "status": "failed",
                "output": "tool failed"
            }
        ]
    });

    let prepared = prepare_provider_request(
        ProviderProtocol::OpenaiResponses,
        payload.clone(),
        "upstream-model",
        &NoopObserver,
    );

    assert_eq!(prepared["model"], "upstream-model");
    assert_eq!(prepared["status"], "completed");
    assert!(prepared["input"][0].get("status").is_none());
    assert!(prepared["input"][1].get("status").is_none());
    for field in ["content", "summary", "encrypted_content"] {
        assert_eq!(prepared["input"][1][field], payload["input"][1][field]);
    }
    assert_eq!(prepared["input"][2]["status"], "failed");
}

#[test]
fn responses_preparation_removes_unsupported_none_effort() {
    let observer = RecordingObserver::default();
    let payload = json!({
        "model": "gpt-6-astra",
        "reasoning": {"effort": "none"}
    });

    let prepared = prepare_provider_request(
        ProviderProtocol::OpenaiResponses,
        payload,
        "upstream-model",
        &observer,
    );

    assert!(prepared.get("reasoning").is_none());
    assert!(
        observer
            .values
            .lock()
            .unwrap()
            .iter()
            .any(|observation| matches!(
                observation,
                Observation::Provider(ProviderObservation::RequestAdapted {
                    adaptation: ProviderRequestAdaptation::OpenaiResponsesOutputFieldsRemoved {
                        reasoning_none_effort_removed: 1,
                        ..
                    },
                    ..
                })
            ))
    );
}

#[test]
fn responses_preparation_emits_typed_request_adaptation() {
    let observer = RecordingObserver::default();
    let recorded = observer.values.clone();
    let payload = json!({
        "input": [
            {"type": "message", "status": "completed"},
            {"type": "reasoning", "status": "completed", "content": []}
        ],
        "reasoning": {"effort": "none", "summary": "auto"}
    });

    let prepared = prepare_provider_request(
        ProviderProtocol::OpenaiResponses,
        payload,
        "upstream-model",
        &observer,
    );

    assert_eq!(prepared["reasoning"], json!({"summary": "auto"}));
    assert!(matches!(
        recorded.lock().unwrap().as_slice(),
        [Observation::Provider(ProviderObservation::RequestAdapted {
            protocol: ProviderProtocol::OpenaiResponses,
            adaptation: ProviderRequestAdaptation::OpenaiResponsesOutputFieldsRemoved {
                status_removed: 2,
                reasoning_none_effort_removed: 1,
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
