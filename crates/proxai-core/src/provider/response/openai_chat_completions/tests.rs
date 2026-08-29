use serde_json::json;

use crate::protocol::openai::chat_completions::{
    CreateChatCompletionResponse, CreateChatCompletionStreamResponse,
};

use super::{normalize_response_payload, normalize_stream_event_payload};

#[test]
fn normalizes_standard_service_tier_in_non_streaming_response() {
    let normalized = normalize_response_payload(json!({
        "id": "chatcmpl_compat",
        "object": "chat.completion",
        "created": 1,
        "model": "MiniMax-M3",
        "choices": [],
        "service_tier": "standard"
    }));

    assert_eq!(normalized["service_tier"], "default");
    serde_json::from_value::<CreateChatCompletionResponse>(normalized)
        .expect("normalized compatible Chat response should match the official wire model");
}

#[test]
fn fills_missing_finish_reason_with_null() {
    let normalized = normalize_stream_event_payload(json!({
        "id": "chatcmpl_minimax",
        "object": "chat.completion.chunk",
        "created": 1,
        "model": "MiniMax-M3",
        "choices": [{
            "index": 0,
            "delta": {"content": "hello"}
        }]
    }));

    assert_eq!(normalized["choices"][0]["finish_reason"], json!(null));
    serde_json::from_value::<CreateChatCompletionStreamResponse>(normalized)
        .expect("normalized compatible Chat chunk should match the official wire model");
}

#[test]
fn preserves_explicit_finish_reason_values() {
    for finish_reason in [json!(null), json!("stop")] {
        let normalized = normalize_stream_event_payload(json!({
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": finish_reason
            }]
        }));

        assert_eq!(normalized["choices"][0]["finish_reason"], finish_reason);
    }
}
