use serde_json::json;

use crate::protocol::openai::chat_completions::CreateChatCompletionRequest;
use crate::protocol::openai::responses::ResponseCreateParams;

use super::super::translate_request_payload;

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

    let translated = translate_request_payload(&payload).unwrap();
    serde_json::from_value::<CreateChatCompletionRequest>(translated.clone())
        .expect("translated payload must match Chat Completions request schema");

    assert_eq!(translated["model"], "glm-5.1");
    assert_eq!(translated["max_completion_tokens"], 128);
    assert_eq!(translated["parallel_tool_calls"], false);
    assert_eq!(translated["reasoning_effort"], "high");
    assert_eq!(translated["stream"], true);
    // top-level `instructions` is prepended as a developer message in modern Chat.
    assert_eq!(translated["messages"][0]["role"], "developer");
    assert_eq!(translated["messages"][0]["content"], "Be concise.");
    // The user message follows the developer message.
    let user = &translated["messages"][1];
    assert_eq!(user["role"], "user");
    assert_eq!(user["content"][0]["type"], "text");
    assert_eq!(user["content"][1]["type"], "image_url");
    // Function call maps to an assistant tool_calls message.
    assert_eq!(translated["messages"][2]["role"], "assistant");
    assert_eq!(
        translated["messages"][2]["tool_calls"][0]["type"],
        "function"
    );
    // Function call output maps to a tool message.
    assert_eq!(translated["messages"][3]["role"], "tool");
    assert_eq!(translated["tools"][0]["type"], "function");
    assert_eq!(translated["tool_choice"], "required");
}

#[test]
fn preserves_responses_system_and_developer_roles_as_chat_instruction_messages() {
    let payload = json!({
        "model": "glm-5.1",
        "instructions": "top-level developer instruction",
        "input": [
            {
                "type": "message",
                "role": "system",
                "content": [{"type": "input_text", "text": "system instruction"}]
            },
            {
                "type": "message",
                "role": "developer",
                "content": [{"type": "input_text", "text": "developer instruction"}]
            },
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "hello"}]
            }
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();
    serde_json::from_value::<CreateChatCompletionRequest>(translated.clone())
        .expect("translated payload must match Chat Completions request schema");

    assert_eq!(translated["messages"][0]["role"], "developer");
    assert_eq!(
        translated["messages"][0]["content"],
        "top-level developer instruction"
    );
    assert_eq!(translated["messages"][1]["role"], "system");
    assert_eq!(translated["messages"][1]["content"], "system instruction");
    assert_eq!(translated["messages"][2]["role"], "developer");
    assert_eq!(
        translated["messages"][2]["content"],
        "developer instruction"
    );
    assert_eq!(translated["messages"][3]["role"], "user");
    assert_eq!(translated["messages"][3]["content"][0]["text"], "hello");
}

#[test]
fn parses_responses_request_into_native_response_create_params() {
    let payload = json!({
        "model": "glm-5.1",
        "instructions": "Be concise.",
        "input": [{"type": "message", "role": "user", "content": "hi"}],
        "max_output_tokens": 16
    });

    serde_json::from_value::<ResponseCreateParams>(payload).unwrap();
}

#[test]
fn translates_item_reference_to_placeholder() {
    // Responses item references are stateful pointers; the proxy cannot resolve
    // them, so they are surfaced as a user-text placeholder so the omission is
    // observable rather than silently dropped.
    let payload = json!({
        "model": "glm-5.1",
        "input": [
            {"type": "item_reference", "id": "msg_abc"}
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();
    serde_json::from_value::<CreateChatCompletionRequest>(translated.clone())
        .expect("translated payload must match Chat Completions request schema");

    assert_eq!(translated["messages"][0]["role"], "user");
    assert!(
        translated["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("item_reference"),
        "unexpected content: {}",
        translated["messages"][0]["content"]
    );
}

#[test]
fn rejects_responses_request_without_model() {
    let payload = json!({
        "input": [{"type": "message", "role": "user", "content": "hi"}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();
    assert!(error.contains("`model`"));
}

#[test]
fn flattens_function_call_arguments_into_assistant_tool_call() {
    let payload = json!({
        "model": "glm-5.1",
        "input": [
            {"type": "function_call", "call_id": "call_1", "name": "lookup", "arguments": "{\"id\":\"42\"}"},
            {"type": "function_call", "call_id": "call_2", "name": "lookup", "arguments": "{}"}
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();
    // Two function calls collapse into a single assistant message with two tool_calls.
    assert_eq!(translated["messages"][0]["role"], "assistant");
    assert_eq!(
        translated["messages"][0]["tool_calls"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn skips_hosted_responses_tools_without_chat_equivalent() {
    let payload = json!({
        "model": "glm-5.1",
        "input": [{"type": "message", "role": "user", "content": "search"}],
        "tools": [
            {"type": "function", "name": "lookup", "parameters": {"type": "object"}},
            {"type": "web_search"}
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();
    let tools = translated["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["type"], "function");
}
