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
    .with_observer(Arc::new(observer));
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
fn reports_request_content_observations_with_wire_discriminants() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiChatCompletions,
    )
    .with_observer(Arc::new(observer));
    let payload = json!({
        "model": "gpt-5.1",
        "input": [{
            "type": "message",
            "role": "system",
            "content": [{
                "type": "input_image",
                "image_url": "https://example.test/image.png"
            }]
        }]
    });

    translator.translate_request(&payload).unwrap();

    assert_eq!(
        observations.lock().unwrap().as_slice(),
        &[
            TranslationObservation {
                request_protocol: RequestProtocol::OpenaiResponses,
                provider_protocol: ProviderProtocol::OpenaiChatCompletions,
                phase: TranslationPhase::Request,
                kind: TranslationObservationKind::Dropped,
                subject: "Responses instruction content `input_image`".to_string(),
                detail: "Chat instruction messages can only represent text".to_string(),
            },
            TranslationObservation {
                request_protocol: RequestProtocol::OpenaiResponses,
                provider_protocol: ProviderProtocol::OpenaiChatCompletions,
                phase: TranslationPhase::Request,
                kind: TranslationObservationKind::Adapted,
                subject: "Responses request without representable messages".to_string(),
                detail:
                    "Chat Completions requires at least one message; emitting an empty user message"
                        .to_string(),
            },
        ]
    );
}

#[test]
fn reports_unrepresentable_responses_input_item_without_injecting_prompt_text() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiChatCompletions,
    )
    .with_observer(Arc::new(observer));

    let translated = translator
        .translate_request(&json!({
            "model": "gpt-5.6",
            "input": [
                {"type": "message", "role": "user", "content": "continue"},
                {
                    "type": "additional_tools",
                    "role": "developer",
                    "tools": []
                }
            ]
        }))
        .unwrap();

    assert_eq!(translated["messages"].as_array().unwrap().len(), 1);
    assert_eq!(translated["messages"][0]["content"], "continue");
    assert_eq!(
        observations.lock().unwrap().as_slice(),
        &[TranslationObservation {
            request_protocol: RequestProtocol::OpenaiResponses,
            provider_protocol: ProviderProtocol::OpenaiChatCompletions,
            phase: TranslationPhase::Request,
            kind: TranslationObservationKind::Dropped,
            subject: "Responses input item `additional_tools`".to_string(),
            detail: "Chat Completions cannot represent per-item developer tool declarations"
                .to_string(),
        }]
    );
}

#[test]
fn reports_responses_original_image_detail_adaptation() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiChatCompletions,
    )
    .with_observer(Arc::new(observer));

    translator
        .translate_request(&json!({
            "model": "gpt-5.6",
            "input": [{
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_image",
                    "image_url": "https://example.test/original.png",
                    "detail": "original"
                }]
            }]
        }))
        .unwrap();

    assert_eq!(
        observations.lock().unwrap().as_slice(),
        &[TranslationObservation {
            request_protocol: RequestProtocol::OpenaiResponses,
            provider_protocol: ProviderProtocol::OpenaiChatCompletions,
            phase: TranslationPhase::Request,
            kind: TranslationObservationKind::Adapted,
            subject: "Responses input_image detail `original`".to_string(),
            detail: "Chat Completions has no `original` image detail; falling back to `auto` while preserving the image".to_string(),
        }]
    );
}

#[test]
fn reports_dropped_responses_output_text_metadata() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiChatCompletions,
    )
    .with_observer(Arc::new(observer));

    translator
        .translate_request(&json!({
            "model": "gpt-5.6",
            "input": [{
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "status": "completed",
                "content": [{
                    "type": "output_text",
                    "text": "answer",
                    "annotations": [{
                        "type": "url_citation",
                        "url": "https://example.test/source",
                        "start_index": 0,
                        "end_index": 6,
                        "title": "source"
                    }],
                    "logprobs": [{
                        "token": "answer",
                        "logprob": -0.1,
                        "bytes": [97],
                        "top_logprobs": []
                    }]
                }]
            }]
        }))
        .unwrap();

    assert_eq!(
        observations.lock().unwrap().as_slice(),
        &[
            TranslationObservation {
                request_protocol: RequestProtocol::OpenaiResponses,
                provider_protocol: ProviderProtocol::OpenaiChatCompletions,
                phase: TranslationPhase::Request,
                kind: TranslationObservationKind::Dropped,
                subject: "Responses output_text annotations".to_string(),
                detail: "Chat assistant history content cannot represent output text annotations"
                    .to_string(),
            },
            TranslationObservation {
                request_protocol: RequestProtocol::OpenaiResponses,
                provider_protocol: ProviderProtocol::OpenaiChatCompletions,
                phase: TranslationPhase::Request,
                kind: TranslationObservationKind::Dropped,
                subject: "Responses output_text logprobs".to_string(),
                detail:
                    "Chat assistant history content cannot represent output token log probabilities"
                        .to_string(),
            },
        ]
    );
}

#[test]
fn reports_custom_tool_output_content_omissions_with_custom_source() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiChatCompletions,
    )
    .with_observer(Arc::new(observer));

    let translated = translator
        .translate_request(&json!({
            "model": "gpt-5.6",
            "input": [
                {
                    "type": "custom_tool_call",
                    "call_id": "call_1",
                    "name": "inspect",
                    "input": "image"
                },
                {
                    "type": "custom_tool_call_output",
                    "call_id": "call_1",
                    "output": [{
                        "type": "input_image",
                        "image_url": "https://example.test/result.png"
                    }]
                }
            ]
        }))
        .unwrap();

    assert_eq!(translated["messages"][1]["content"], "");
    assert_eq!(
        observations.lock().unwrap().as_slice(),
        &[TranslationObservation {
            request_protocol: RequestProtocol::OpenaiResponses,
            provider_protocol: ProviderProtocol::OpenaiChatCompletions,
            phase: TranslationPhase::Request,
            kind: TranslationObservationKind::Dropped,
            subject: "Responses custom_tool_call_output `input_image`".to_string(),
            detail: "Chat tool messages can only represent text output content".to_string(),
        }]
    );
}

#[test]
fn reports_encrypted_reasoning_dropped_while_preserving_visible_text() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiChatCompletions,
    )
    .with_observer(Arc::new(observer));

    let translated = translator
        .translate_request(&json!({
            "model": "gpt-5.6",
            "input": [{
                "type": "reasoning",
                "id": "rs_1",
                "encrypted_content": "opaque-state",
                "summary": [{"type": "summary_text", "text": "visible reasoning"}],
                "status": "completed"
            }]
        }))
        .unwrap();

    assert_eq!(
        translated["messages"][0]["reasoning_content"],
        "visible reasoning"
    );
    assert_eq!(
        observations.lock().unwrap().as_slice(),
        &[TranslationObservation {
            request_protocol: RequestProtocol::OpenaiResponses,
            provider_protocol: ProviderProtocol::OpenaiChatCompletions,
            phase: TranslationPhase::Request,
            kind: TranslationObservationKind::Dropped,
            subject: "Responses reasoning encrypted_content".to_string(),
            detail: "Chat reasoning_content preserves visible reasoning text but cannot represent encrypted reasoning state"
                .to_string(),
        }]
    );
}

#[test]
fn reports_responses_image_file_id_adaptation() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiChatCompletions,
    )
    .with_observer(Arc::new(observer));

    translator
        .translate_request(&json!({
            "model": "gpt-5.6",
            "input": [{
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_image",
                    "file_id": "file_image_1"
                }]
            }]
        }))
        .unwrap();

    assert_eq!(
        observations.lock().unwrap().as_slice(),
        &[TranslationObservation {
            request_protocol: RequestProtocol::OpenaiResponses,
            provider_protocol: ProviderProtocol::OpenaiChatCompletions,
            phase: TranslationPhase::Request,
            kind: TranslationObservationKind::Adapted,
            subject: "Responses input_image.file_id".to_string(),
            detail: "Chat image content accepts only image_url; projecting the uploaded image as Chat file content".to_string(),
        }]
    );
}

#[test]
fn reports_responses_file_url_omission_without_failing_translation() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiChatCompletions,
    )
    .with_observer(Arc::new(observer));

    translator
        .translate_request(&json!({
            "model": "gpt-5.6",
            "input": [{
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_file",
                    "file_url": "https://example.test/file.pdf"
                }]
            }]
        }))
        .unwrap();

    assert_eq!(
        observations.lock().unwrap().as_slice(),
        &[
            TranslationObservation {
                request_protocol: RequestProtocol::OpenaiResponses,
                provider_protocol: ProviderProtocol::OpenaiChatCompletions,
                phase: TranslationPhase::Request,
                kind: TranslationObservationKind::Dropped,
                subject: "Responses input_file.file_url".to_string(),
                detail: "Chat file content supports file_id or file_data, not file_url".to_string(),
            },
            TranslationObservation {
                request_protocol: RequestProtocol::OpenaiResponses,
                provider_protocol: ProviderProtocol::OpenaiChatCompletions,
                phase: TranslationPhase::Request,
                kind: TranslationObservationKind::Dropped,
                subject: "Responses input_file content".to_string(),
                detail: "Chat file content requires file_id or file_data".to_string(),
            },
            TranslationObservation {
                request_protocol: RequestProtocol::OpenaiResponses,
                provider_protocol: ProviderProtocol::OpenaiChatCompletions,
                phase: TranslationPhase::Request,
                kind: TranslationObservationKind::Adapted,
                subject: "Responses request without representable messages".to_string(),
                detail:
                    "Chat Completions requires at least one message; emitting an empty user message"
                        .to_string(),
            },
        ]
    );
}

#[test]
fn reports_minimal_reasoning_effort_adaptation() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::AnthropicMessages,
    )
    .with_observer(Arc::new(observer));

    translator
        .translate_request(&json!({
            "model": "gpt-5.5",
            "input": "hello",
            "reasoning": {"effort": "minimal"}
        }))
        .unwrap();

    assert_eq!(
        observations.lock().unwrap().as_slice(),
        &[TranslationObservation {
            request_protocol: RequestProtocol::OpenaiResponses,
            provider_protocol: ProviderProtocol::AnthropicMessages,
            phase: TranslationPhase::Request,
            kind: TranslationObservationKind::Adapted,
            subject: "Responses reasoning.effort `minimal`".to_string(),
            detail: "Anthropic output_config.effort has no minimal level; using low".to_string(),
        }]
    );
}

#[test]
fn binds_response_observations_to_non_streaming_response_phase() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::AnthropicMessages,
        ProviderProtocol::OpenaiChatCompletions,
    )
    .with_observer(Arc::new(observer));
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

    assert_eq!(output.len(), 2);
    let event = output[0].as_ref().unwrap();
    assert_eq!(event.event_type, "message");
    assert_eq!(event.data, json!({"id": "chatcmpl_1"}));
    assert!(output[1].as_ref().unwrap().is_done_sentinel());
}

#[tokio::test]
async fn reports_unrepresentable_stream_output_through_observer() {
    let observer = RecordingObserver::default();
    let observations = observer.observations.clone();
    let translator = Translator::new(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiResponses,
    )
    .with_observer(Arc::new(observer));
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
