use serde_json::json;

use crate::protocol::openai_responses::ResponseCreateParams;

use super::super::translate_request_payload;

#[test]
fn translates_chat_completions_request_to_responses_shape() {
    let payload = json!({
        "model": "gpt-5.1",
        "messages": [
            {"role": "system", "content": "Be concise."},
            {"role": "developer", "content": [{"type": "text", "text": "Prefer JSON."}]},
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "hello"},
                    {"type": "image_url", "image_url": {"url": "https://example.test/a.png", "detail": "low"}},
                    {"type": "file", "file": {"file_id": "file_123", "filename": "notes.pdf"}}
                ]
            },
            {
                "role": "assistant",
                "content": "checking",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "lookup", "arguments": "{\"id\":\"42\"}"}
                }]
            },
            {"role": "tool", "tool_call_id": "call_1", "content": "result"}
        ],
        "max_completion_tokens": 128,
        "parallel_tool_calls": false,
        "reasoning_effort": "high",
        "response_format": {"type": "json_object"},
        "tool_choice": {"type": "function", "function": {"name": "lookup"}},
        "tools": [{
            "type": "function",
            "function": {
                "name": "lookup",
                "description": "Look up a record",
                "parameters": {"type": "object", "properties": {"id": {"type": "string"}}},
                "strict": true
            }
        }],
        "stream": true,
        "stream_options": {"include_usage": true, "include_obfuscation": false},
        "temperature": 1.0,
        "top_p": 0.9
    });

    let translated = translate_request_payload(&payload).unwrap();
    serde_json::from_value::<ResponseCreateParams>(translated.clone())
        .expect("translated payload must match Responses request schema");

    assert_eq!(translated["model"], "gpt-5.1");
    assert_eq!(translated["instructions"], "Be concise.\n\nPrefer JSON.");
    assert_eq!(translated["max_output_tokens"], 128);
    assert_eq!(translated["parallel_tool_calls"], false);
    assert_eq!(translated["reasoning"]["effort"], "high");
    assert_eq!(translated["text"]["format"]["type"], "json_object");
    assert_eq!(translated["stream"], true);
    assert_eq!(translated["stream_options"]["include_obfuscation"], false);
    assert_eq!(translated["input"][0]["role"], "user");
    assert_eq!(translated["input"][0]["content"][0]["type"], "input_text");
    assert_eq!(translated["input"][0]["content"][1]["type"], "input_image");
    assert_eq!(translated["input"][0]["content"][2]["type"], "input_file");
    assert_eq!(translated["input"][1]["role"], "assistant");
    assert_eq!(translated["input"][1]["content"][0]["type"], "output_text");
    assert_eq!(translated["input"][2]["type"], "function_call");
    assert_eq!(translated["input"][2]["name"], "lookup");
    assert_eq!(translated["input"][3]["type"], "function_call_output");
    assert_eq!(translated["tools"][0]["type"], "function");
    assert_eq!(translated["tool_choice"]["name"], "lookup");
}

#[test]
fn preserves_chat_assistant_refusal_content_for_responses_input() {
    let payload = json!({
        "model": "gpt-5.1",
        "messages": [{
            "role": "assistant",
            "content": [{"type": "refusal", "refusal": "I can't help with that."}]
        }]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["input"][0]["role"], "assistant");
    assert_eq!(translated["input"][0]["content"][0]["type"], "refusal");
    assert_eq!(
        translated["input"][0]["content"][0]["refusal"],
        "I can't help with that."
    );
}

#[test]
fn preserves_chat_tool_message_array_as_function_call_output_content() {
    let payload = json!({
        "model": "gpt-5.1",
        "messages": [
            {
                "role": "assistant",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "lookup", "arguments": "{}"}
                }]
            },
            {
                "role": "tool",
                "tool_call_id": "call_1",
                "content": [
                    {"type": "text", "text": "line 1"},
                    {"type": "text", "text": "line 2"}
                ]
            }
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["input"][1]["type"], "function_call_output");
    assert_eq!(translated["input"][1]["output"][0]["type"], "input_text");
    assert_eq!(translated["input"][1]["output"][0]["text"], "line 1");
    assert_eq!(translated["input"][1]["output"][1]["text"], "line 2");
}

#[test]
fn rejects_chat_request_without_non_system_messages() {
    let payload = json!({
        "model": "gpt-5.1",
        "messages": [{"role": "system", "content": "Be concise."}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("without user, assistant, or tool messages"));
}

#[test]
fn rejects_empty_chat_assistant_message_for_request_translation() {
    let payload = json!({
        "model": "gpt-5.1",
        "messages": [{"role": "assistant"}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("assistant message without content, refusal, or tool calls"));
}

#[test]
fn rejects_empty_chat_user_text_for_request_translation() {
    let payload = json!({
        "model": "gpt-5.1",
        "messages": [{"role": "user", "content": ""}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("user message text content cannot be empty"));
}

#[test]
fn flattens_same_mode_allowed_tools_tool_choice_for_request_translation() {
    let payload = json!({
        "model": "gpt-5.1",
        "messages": [{"role": "user", "content": "hi"}],
        "tool_choice": {
            "allowed_tools": [
                {"mode": "Auto", "tools": [{"type": "function", "name": "a"}]},
                                {"mode": "Auto", "tools": [{"type": "function", "name": "b"}]}
            ]
        }
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["tool_choice"]["mode"], "auto");
    assert_eq!(translated["tool_choice"]["tools"][0]["name"], "a");
    assert_eq!(translated["tool_choice"]["tools"][1]["name"], "b");
}

#[test]
fn rejects_mixed_mode_allowed_tools_tool_choice_for_request_translation() {
    let payload = json!({
        "model": "gpt-5.1",
        "messages": [{"role": "user", "content": "hi"}],
        "tool_choice": {
            "allowed_tools": [
                {"mode": "Auto", "tools": [{"type": "function", "name": "a"}]},
                                {"mode": "Required", "tools": [{"type": "function", "name": "b"}]}
            ]
        }
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("allowed_tools tool_choice cannot mix modes"));
}

#[test]
fn rejects_legacy_function_messages_for_request_translation() {
    let payload = json!({
        "model": "gpt-5.1",
        "messages": [{"role": "function", "name": "lookup", "content": "result"}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("legacy function message"));
}
