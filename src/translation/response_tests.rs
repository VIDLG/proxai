use serde_json::json;

use crate::http_support::into_byte_stream;
use crate::protocol::{ProviderProtocol, RequestProtocol};

use super::{translate_non_streaming_response, translate_streaming_response};

#[test]
fn passes_through_self_protocol_non_streaming_payload() {
    let payload = json!({"error": "upstream failed"});

    let translated = translate_non_streaming_response(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiResponses,
        payload.clone(),
    )
    .unwrap();

    assert_eq!(translated, payload);
}

#[test]
fn supports_responses_to_chat_completions_non_streaming_translation() {
    // Previously unsupported; now implemented via responses → chat translator.
    let payload = json!({
        "id": "resp_1",
        "model": "glm-5.1",
        "created_at": 0,
        "status": "completed",
        "object": "response",
        "output": [
            {"type": "message", "id": "m", "role": "assistant", "status": "completed", "content": [{"type": "output_text", "text": "hi", "annotations": []}]}
        ]
    });

    let translated = translate_non_streaming_response(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiResponses,
        payload,
    )
    .unwrap();

    assert_eq!(translated["object"], "chat.completion");
    assert_eq!(translated["choices"][0]["message"]["content"], "hi");
}

#[test]
fn supports_chat_completions_to_responses_streaming_translation() {
    // Previously unsupported; now implemented via responses → chat translator.
    let result = translate_streaming_response(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiResponses,
        into_byte_stream(axum::body::Body::empty().into_data_stream()),
    );

    // An empty body produces no events and succeeds; the important assertion
    // is that the pair is no longer rejected as unsupported.
    assert!(result.is_ok());
}
