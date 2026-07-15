use std::sync::{Arc, Mutex};

use futures_util::{StreamExt, stream};
use proxai_core::ingress::prepare_inbound_request;
use proxai_core::observe::{IngressObservation, NoopObserver, Observation, Observer};
use proxai_core::protocol::{ProviderProtocol, RequestProtocol};
use proxai_core::translation::Translator;
use proxai_core::translation::stream::{StreamEnd, StreamEvent, StreamTranslationInput};
use serde_json::json;

#[derive(Clone, Default)]
struct RecordingObserver {
    observations: Arc<Mutex<Vec<Observation>>>,
}

impl Observer for RecordingObserver {
    fn observe(&self, observation: &Observation) {
        self.observations.lock().unwrap().push(observation.clone());
    }
}

#[test]
fn reports_ingress_observations_to_a_downstream_implementation() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();

    prepare_inbound_request(
        RequestProtocol::AnthropicMessages,
        json!({
            "model": "claude-test",
            "max_tokens": 256,
            "messages": [{"role": "user", "content": "hello"}],
            "thinking": {"type": "enabled", "budget_tokens": 1024}
        }),
        &observer,
    )
    .unwrap();

    assert_eq!(
        observations.lock().unwrap().as_slice(),
        &[Observation::Ingress(
            IngressObservation::AnthropicLegacyThinkingBudget {
                model: "claude-test".to_string(),
                budget_tokens: 1024,
            }
        )]
    );
}

#[test]
fn prepares_and_translates_values_through_the_public_api() {
    let request = prepare_inbound_request(
        RequestProtocol::OpenaiResponses,
        json!({
            "model": "gpt-5.1",
            "input": [{"role": "system", "content": "be concise"}, {"role": "user", "content": "hello"}]
        }),
        &NoopObserver,
    )
    .unwrap();
    assert_eq!(request.model(), "gpt-5.1");
    assert_eq!(request.normalized_payload()["instructions"], "be concise");

    let translator = Translator::new(request.protocol(), ProviderProtocol::OpenaiChatCompletions);
    let translated = translator
        .translate_request(request.normalized_payload())
        .unwrap();

    assert_eq!(translated["model"], "gpt-5.1");
    assert_eq!(translated["messages"][0]["role"], "developer");
}

#[test]
fn translates_values_through_the_public_api() {
    let translator = Translator::new(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiResponses,
    );
    let request = json!({
        "model": "gpt-5.1",
        "messages": [{"role": "user", "content": "hello"}]
    });

    let translated = translator.translate_request(&request).unwrap();

    assert_eq!(translated["model"], "gpt-5.1");
    assert_eq!(translated["input"][0]["role"], "user");
}

#[tokio::test]
async fn translates_structured_streams_through_the_public_api() {
    let translator = Translator::new(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiChatCompletions,
    );
    let input = stream::iter([
        Ok(StreamTranslationInput::Event(StreamEvent::new(
            "message",
            json!({"id": "chatcmpl_1"}),
        ))),
        Ok(StreamTranslationInput::End(StreamEnd::Done)),
    ]);

    let output = translator.translate_stream(input).collect::<Vec<_>>().await;

    assert_eq!(output.len(), 2);
    assert_eq!(output[0].as_ref().unwrap().data["id"], "chatcmpl_1");
    assert!(output[1].as_ref().unwrap().is_done_sentinel());
}
