use async_openai::types::chat::CreateChatCompletionRequest;
use serde_json::json;

use super::translate_request_payload;

#[test]
fn translates_responses_request_to_chat_completions_shape() {
    let payload = json!({
        "model": "glm-5.1",
        "instructions": "Be concise.",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    {"type": "input_text", "text": "hello"},
                    {"type": "input_image", "image_url": "https://example.test/a.png", "detail": "low"}
                ]
            },
            {"type": "function_call", "call_id": "call_1", "name": "lookup", "arguments": "{\"id\":\"42\"}"},
            {"type": "function_call_output", "call_id": "call_1", "output": "result"}
        ],
        "max_output_tokens": 128,
        "parallel_tool_calls": false,
        "reasoning": {"effort": "high"},
        "tool_choice": "required",
        "tools": [{
            "type": "function",
            "name": "lookup",
            "description": "Look up a record",
            "parameters": {"type": "object", "properties": {"id": {"type": "string"}}}
        }],
        "stream": true,
        "temperature": 1.0,
        "top_p": 0.9
    });

    let translated = translate_request_payload(&payload, "glm-5.1", "MiniMax-M3").unwrap();
    serde_json::from_value::<CreateChatCompletionRequest>(translated.clone())
        .expect("translated payload must match Chat Completions request schema");

    assert_eq!(translated["model"], "MiniMax-M3");
    assert_eq!(translated["max_completion_tokens"], 128);
    assert_eq!(translated["parallel_tool_calls"], false);
    assert_eq!(translated["reasoning_effort"], "high");
    assert_eq!(translated["stream"], true);
    assert_eq!(translated["messages"][0]["role"], "system");
    assert_eq!(translated["messages"][0]["content"], "Be concise.");
    assert_eq!(translated["messages"][1]["role"], "user");
    assert_eq!(translated["messages"][1]["content"][0]["type"], "text");
    assert_eq!(translated["messages"][1]["content"][1]["type"], "image_url");
    assert_eq!(translated["messages"][2]["role"], "assistant");
    assert_eq!(
        translated["messages"][2]["tool_calls"][0]["type"],
        "function"
    );
    assert_eq!(translated["messages"][3]["role"], "tool");
    assert_eq!(translated["tools"][0]["type"], "function");
    assert_eq!(translated["tool_choice"], "required");
}

#[test]
fn translates_unknown_responses_input_item_to_placeholder() {
    let payload = json!({
        "model": "glm-5.1",
        "input": [
            {"type": "future_zed_item", "opaque": {"value": 1}}
        ]
    });

    let translated = translate_request_payload(&payload, "glm-5.1", "glm-5.1").unwrap();
    serde_json::from_value::<CreateChatCompletionRequest>(translated.clone())
        .expect("translated payload must match Chat Completions request schema");

    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(
        translated["messages"][0]["content"],
        "[OpenAI Responses item `future_zed_item` omitted during Chat Completions translation]"
    );
}
