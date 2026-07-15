use std::sync::{Arc, Mutex};

use futures_util::{StreamExt, stream};
use serde_json::{Value, json};

use super::Translator;
use crate::observe::{
    Observation, Observer, TranslationObservation, TranslationObservationKind, TranslationPhase,
};
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::stream::{StreamEnd, StreamEvent, StreamTranslationInput};

#[derive(Clone, Default)]
struct RecordingObserver {
    observations: Arc<Mutex<Vec<TranslationObservation>>>,
}

impl Observer for RecordingObserver {
    fn observe(&self, observation: &Observation) {
        if let Observation::Translation(observation) = observation {
            self.observations.lock().unwrap().push(observation.clone());
        }
    }
}

#[test]
fn preserves_identity_request_payload() {
    let translator = Translator::new(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiResponses,
    );
    let payload = json!({"model": "gpt-5.1", "input": "hello"});

    assert_eq!(translator.translate_request(&payload).unwrap(), payload);
}

#[test]
fn preserves_identity_response_payload() {
    let translator = Translator::new(
        RequestProtocol::AnthropicMessages,
        ProviderProtocol::AnthropicMessages,
    );
    let payload = json!({"type": "message", "id": "msg_1", "content": []});

    assert_eq!(
        translator.translate_response(payload.clone()).unwrap(),
        payload
    );
}

#[test]
fn translates_cross_protocol_request_through_facade() {
    let translator = Translator::new(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiResponses,
    );
    let payload = json!({
        "model": "gpt-5.1",
        "messages": [{"role": "user", "content": "hello"}]
    });

    let translated = translator.translate_request(&payload).unwrap();

    assert_eq!(translated["model"], "gpt-5.1");
    assert_eq!(translated["input"][0]["role"], "user");
    assert_eq!(translated["input"][0]["content"], "hello");
}

#[test]
fn binds_request_observations_to_request_phase() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiResponses,
    )
    .with_observer(observer);
    let payload = json!({
        "model": "gpt-5.1",
        "messages": [{"role": "user", "content": "hello"}],
        "seed": 42
    });

    translator.translate_request(&payload).unwrap();

    let observations = observations.lock().unwrap();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].phase, TranslationPhase::Request);
}

#[test]
fn binds_response_observations_to_non_streaming_response_phase() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::AnthropicMessages,
        ProviderProtocol::OpenaiChatCompletions,
    )
    .with_observer(observer);
    let payload = json!({
        "id": "chatcmpl_reasoning",
        "object": "chat.completion",
        "created": 1234,
        "model": "gpt-test",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "answer",
                "reasoning_content": "unsigned thinking",
                "refusal": null
            },
            "finish_reason": "stop",
            "logprobs": null
        }]
    });

    translator.translate_response(payload).unwrap();

    let observations = observations.lock().unwrap();
    assert_eq!(observations.len(), 1);
    assert_eq!(
        observations[0].phase,
        TranslationPhase::NonStreamingResponse
    );
}

#[tokio::test]
async fn preserves_identity_structured_stream() {
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

    assert_eq!(output.len(), 1);
    let event = output[0].as_ref().unwrap();
    assert_eq!(event.event_type, "message");
    assert_eq!(event.data, json!({"id": "chatcmpl_1"}));
}

#[tokio::test]
async fn reports_unrepresentable_stream_output_through_observer() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiResponses,
    )
    .with_observer(observer);
    let input = stream::iter([
        Ok(StreamTranslationInput::Event(StreamEvent::new(
            "response.created",
            response_created_event(),
        ))),
        Ok(StreamTranslationInput::Event(StreamEvent::new(
            "response.output_item.added",
            json!({
                "type": "response.output_item.added",
                "sequence_number": 2,
                "output_index": 0,
                "item": {
                    "type": "file_search_call",
                    "id": "fs_1",
                    "status": "in_progress",
                    "queries": [],
                    "results": null
                }
            }),
        ))),
        Ok(StreamTranslationInput::End(StreamEnd::Eof)),
    ]);

    let _ = translator.translate_stream(input).collect::<Vec<_>>().await;

    assert_eq!(
        observations.lock().unwrap().as_slice(),
        &[TranslationObservation {
            request_protocol: RequestProtocol::OpenaiChatCompletions,
            provider_protocol: ProviderProtocol::OpenaiResponses,
            phase: TranslationPhase::StreamingResponse,
            kind: TranslationObservationKind::Dropped,
            subject: "Responses output item `file_search_call` at index 0".to_string(),
            detail: "Responses output item has no Chat Completions streaming representation"
                .to_string(),
        }]
    );
}

fn response_created_event() -> Value {
    json!({
        "type": "response.created",
        "sequence_number": 1,
        "response": {
            "id": "resp_1",
            "object": "response",
            "created_at": 0,
            "model": "gpt-5.1",
            "output": [],
            "parallel_tool_calls": false,
            "tool_choice": "auto",
            "tools": [],
            "status": "in_progress",
            "metadata": null,
            "temperature": null,
            "top_p": null,
            "error": null,
            "incomplete_details": null,
            "instructions": null
        }
    })
}
