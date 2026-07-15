use serde_json::json;

use crate::protocol::openai::chat_completions::CreateChatCompletionStreamResponse;

use super::normalize_stream_event_payload;

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
