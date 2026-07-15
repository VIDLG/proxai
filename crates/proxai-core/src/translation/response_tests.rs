use serde_json::json;

use crate::protocol::{ProviderProtocol, RequestProtocol};

use crate::translation::Translator;

#[test]
fn passes_through_self_protocol_non_streaming_payload() {
    let payload = json!({"error": "upstream failed"});

    let translator = Translator::new(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiResponses,
    );
    let translated = translator.translate_response(payload.clone()).unwrap();

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
        "metadata": null,
        "temperature": null,
        "top_p": null,
        "error": null,
        "incomplete_details": null,
        "instructions": null,
        "parallel_tool_calls": false,
        "tool_choice": "auto",
        "tools": [],
        "output": [
            {"type": "message", "id": "m", "role": "assistant", "status": "completed", "content": [{"type": "output_text", "text": "hi", "annotations": [], "logprobs": []}]}
        ]
    });

    let translator = Translator::new(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiResponses,
    );
    let translated = translator.translate_response(payload).unwrap();

    assert_eq!(translated["object"], "chat.completion");
    assert_eq!(translated["choices"][0]["message"]["content"], "hi");
}
