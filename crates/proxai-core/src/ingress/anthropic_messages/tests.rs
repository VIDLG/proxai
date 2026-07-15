use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use crate::ingress::{IngressError, PreparedInboundRequest};
use crate::observe::{IngressObservation, NoopObserver, Observation, Observer};

use super::prepare_anthropic_messages_request;

#[derive(Clone, Default)]
struct RecordingObserver {
    observations: Arc<Mutex<Vec<Observation>>>,
}

impl Observer for RecordingObserver {
    fn observe(&self, observation: &Observation) {
        self.observations.lock().unwrap().push(observation.clone());
    }
}

fn prepare(payload: Value) -> Result<PreparedInboundRequest, IngressError> {
    prepare_anthropic_messages_request(payload, &NoopObserver)
}

#[test]
fn preserves_valid_anthropic_messages_request_verbatim() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 256,
        "messages": [{"role": "user", "content": "hello"}],
        "stream": true
    });

    let prepared = prepare(payload.clone()).unwrap();

    assert_eq!(prepared.model(), "claude-sonnet-4-5");
    assert_eq!(prepared.normalized_payload(), &payload);
}

#[test]
fn rejects_anthropic_messages_request_without_messages() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 256
    });

    let error = prepare(payload).unwrap_err();

    let IngressError::JsonPayload(error) = error else {
        panic!("expected a typed JSON payload error");
    };
    assert_eq!(error.path(), ".");
    assert!(error.to_string().contains("messages"));
}

#[test]
fn rejects_anthropic_messages_request_without_model() {
    let payload = json!({
        "max_tokens": 256,
        "messages": [{"role": "user", "content": "hello"}]
    });

    let error = prepare(payload).unwrap_err();

    let IngressError::JsonPayload(error) = error else {
        panic!("expected a typed JSON payload error");
    };
    assert_eq!(error.path(), ".");
    assert!(error.to_string().contains("model"));
}

#[test]
fn does_not_repair_null_anthropic_messages() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 256,
        "messages": null
    });

    let error = prepare(payload).unwrap_err();

    let IngressError::JsonPayload(error) = error else {
        panic!("expected a typed JSON payload error");
    };
    assert_eq!(error.path(), "messages");
}

#[test]
fn observes_legacy_enabled_thinking_budget_without_rewriting_the_request() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 256,
        "messages": [{"role": "user", "content": "hello"}],
        "thinking": {"type": "enabled", "budget_tokens": 1024}
    });
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();

    let prepared = prepare_anthropic_messages_request(payload.clone(), &observer).unwrap();

    assert_eq!(prepared.model(), "claude-sonnet-4-5");
    assert_eq!(prepared.normalized_payload(), &payload);
    assert_eq!(
        observations.lock().unwrap().as_slice(),
        &[Observation::Ingress(
            IngressObservation::AnthropicLegacyThinkingBudget {
                model: "claude-sonnet-4-5".to_string(),
                budget_tokens: 1024,
            }
        )]
    );
}
