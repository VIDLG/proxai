use serde_json::json;

use crate::protocol::anthropic::messages::MessageCreateParamsBase;

use super::translate_request_payload;

#[test]
fn translates_chat_request_to_anthropic_messages_shape() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [
            {"role": "system", "content": "You are concise."},
            {"role": "developer", "content": [{"type": "text", "text": "Prefer exact answers."}]},
            {"role": "user", "content": [{"type": "text", "text": "Call the tool."}]},
            {
                "role": "assistant",
                "content": "Sure.",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "lookup", "arguments": "{\"query\":\"proxai\"}"}
                }]
            },
            {"role": "tool", "tool_call_id": "call_1", "content": "found"}
        ],
        "tools": [{
            "type": "function",
            "function": {
                "name": "lookup",
                "description": "Lookup a value",
                "parameters": {"properties": {"query": {"type": "string"}}}
            }
        }],
        "tool_choice": {"type": "function", "function": {"name": "lookup"}},
        "stream": false,
        "max_completion_tokens": 128,
        "temperature": 0.2,
        "top_p": 0.9
    });

    let translated = translate_request_payload(&payload).unwrap();
    serde_json::from_value::<MessageCreateParamsBase>(translated.clone())
        .expect("translated payload must match Anthropic Messages request schema");

    assert_eq!(translated["model"], "glm-5.1");
    assert_eq!(translated["max_tokens"], 128);
    assert_eq!(
        translated["system"],
        json!([
            {"type": "text", "text": "You are concise."},
            {"type": "text", "text": "Prefer exact answers."}
        ])
    );
    assert_eq!(translated["stream"], false);
    assert!((translated["temperature"].as_f64().unwrap() - 0.2).abs() < 0.000001);
    assert!((translated["top_p"].as_f64().unwrap() - 0.9).abs() < 0.000001);
    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(
        translated["messages"][0]["content"][0],
        json!({
            "type": "text",
            "text": "Call the tool."
        })
    );
    assert_eq!(
        translated["messages"][1]["content"][1],
        json!({
            "type": "tool_use",
            "id": "call_1",
            "name": "lookup",
            "input": {"query": "proxai"}
        })
    );
    assert_eq!(
        translated["messages"][2]["content"][0],
        json!({
            "type": "tool_result",
            "tool_use_id": "call_1",
            "content": "found",
            "is_error": false
        })
    );
    assert_eq!(translated["tools"][0]["type"], "custom");
    assert_eq!(translated["tools"][0]["name"], "lookup");
    assert_eq!(
        translated["tools"][0]["input_schema"],
        json!({
            "type": "object",
            "properties": {"query": {"type": "string"}},
            "required": []
        })
    );
    assert_eq!(
        translated["tool_choice"],
        json!({"type": "tool", "name": "lookup"})
    );
}

#[test]
fn omits_empty_chat_system_blocks_for_anthropic_messages() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [
            {"role": "system", "content": "  "},
            {"role": "developer", "content": [{"type": "text", "text": ""}]},
            {"role": "user", "content": "hello"}
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert!(translated.get("system").is_none());
}

#[test]
fn rejects_invalid_chat_tool_call_arguments_for_anthropic_messages() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{
            "role": "assistant",
            "tool_calls": [{
                "id": "call_1",
                "type": "function",
                "function": {"name": "lookup", "arguments": "not json"}
            }]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("tool call `call_1` arguments must be valid JSON"));
}

#[test]
fn preserves_chat_tool_message_array_parts_as_anthropic_tool_result_blocks() {
    let payload = json!({
        "model": "glm-5.1",
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
                    {"type": "text", "text": "found"},
                    {"type": "text", "text": " it"}
                ]
            }
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(
        translated["messages"][1]["content"][0]["content"],
        json!([
            {"type": "text", "text": "found"},
            {"type": "text", "text": " it"}
        ])
    );
}

#[test]
fn rejects_empty_chat_user_content_for_anthropic_messages() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "user", "content": ""}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("user message without content"));
}

#[test]
fn rejects_empty_chat_assistant_content_for_anthropic_messages() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "assistant", "content": ""}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("assistant message without content or tool calls"));
}

#[test]
fn rejects_chat_request_without_non_system_messages_for_anthropic_messages() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "system", "content": "Only instructions."}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("at least one non-system message"));
}

#[test]
fn translates_single_chat_assistant_text_to_anthropic_text_content() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "assistant", "content": "Sure."}]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["messages"][0]["role"], "assistant");
    assert_eq!(translated["messages"][0]["content"], "Sure.");
}

#[test]
fn keeps_single_chat_assistant_tool_call_as_anthropic_blocks() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{
            "role": "assistant",
            "tool_calls": [{
                "id": "call_1",
                "type": "function",
                "function": {"name": "lookup", "arguments": "{\"id\":1}"}
            }]
        }]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(
        translated["messages"][0]["content"],
        json!([{
            "type": "tool_use",
            "id": "call_1",
            "name": "lookup",
            "input": {"id": 1}
        }])
    );
}

#[test]
fn rejects_chat_assistant_refusal_content_for_anthropic_messages() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{
            "role": "assistant",
            "content": [{"type": "refusal", "refusal": "I can't help with that."}]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("assistant refusal content cannot be translated"));
}

#[test]
fn translates_chat_pdf_file_url_to_anthropic_document() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{
            "role": "user",
            "content": [{
                "type": "file",
                "file": {
                    "file_data": "https://example.test/report.pdf?download=1",
                    "filename": "report.pdf"
                }
            }]
        }]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(
        translated["messages"][0]["content"][0],
        json!({
            "type": "document",
            "source": {
                "type": "url",
                "url": "https://example.test/report.pdf?download=1"
            },
            "title": "report.pdf"
        })
    );
}

#[test]
fn rejects_chat_file_id_without_file_data_for_anthropic_document() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{
            "role": "user",
            "content": [{
                "type": "file",
                "file": {"file_id": "file_123", "filename": "report.pdf"}
            }]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("only file_id cannot be translated"));
}

#[test]
fn rejects_bare_chat_file_data_for_anthropic_document() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{
            "role": "user",
            "content": [{
                "type": "file",
                "file": {"file_data": "JVBERi0xLjQ=", "filename": "report.pdf"}
            }]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("PDF data URL or PDF URL"));
}

#[test]
fn rejects_legacy_chat_function_messages_for_anthropic_messages() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "function", "name": "lookup", "content": "found"}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("legacy function messages cannot be translated"));
}

#[test]
fn rejects_chat_custom_tools_for_anthropic_messages() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "user", "content": "hello"}],
        "tools": [{
            "type": "custom",
            "custom": {
                "name": "shell",
                "description": "Run shell commands",
                "format": "Text"
            }
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("custom tools cannot be translated"));
}

#[test]
fn translates_chat_tool_choice_none_to_anthropic_tool_choice_none() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "user", "content": "hello"}],
        "tools": [{
            "type": "function",
            "function": {"name": "lookup", "parameters": {"type": "object"}}
        }],
        "tool_choice": "none"
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["tool_choice"], json!({"type": "none"}));
}

#[test]
fn rejects_chat_allowed_tools_choice_for_anthropic_messages() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "user", "content": "hello"}],
        "tool_choice": {
            "allowed_tools": [{
                "mode": "Auto",
                "tools": [{"type": "function", "function": {"name": "lookup"}}]
            }]
        }
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("allowed_tools tool choices cannot be translated"));
}

#[test]
fn rejects_chat_custom_tool_choice_for_anthropic_messages() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "user", "content": "hello"}],
        "tool_choice": {"type": "custom", "custom": {"name": "shell"}}
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("custom tool choices cannot be translated"));
}

#[test]
fn rejects_chat_custom_tool_calls_for_anthropic_messages() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{
            "role": "assistant",
            "tool_calls": [{
                "id": "call_1",
                "type": "custom",
                "custom_tool": {"name": "shell", "input": "pwd"}
            }]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("custom tool calls cannot be translated"));
}

#[test]
fn translates_chat_reasoning_effort_to_anthropic_output_config() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "user", "content": "think"}],
        "reasoning_effort": "high"
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["output_config"], json!({"effort": "high"}));
    assert!(translated.get("thinking").is_none());
}

#[test]
fn omits_anthropic_output_config_for_chat_reasoning_effort_none() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "user", "content": "think"}],
        "reasoning_effort": "none"
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert!(translated.get("output_config").is_none());
    assert_eq!(translated["thinking"], json!({"type": "disabled"}));
}

#[test]
fn disables_anthropic_thinking_for_chat_reasoning_effort_minimal() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "user", "content": "think"}],
        "reasoning_effort": "minimal"
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["output_config"], json!({"effort": "low"}));
    assert_eq!(translated["thinking"], json!({"type": "disabled"}));
}

#[test]
fn prefers_max_completion_tokens_over_deprecated_max_tokens() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "user", "content": "hello"}],
        "max_completion_tokens": 128,
        "max_tokens": 64
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["max_tokens"], 128);
}

#[test]
fn falls_back_to_deprecated_max_tokens() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "user", "content": "hello"}],
        "max_tokens": 64
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["max_tokens"], 64);
}

#[test]
fn uses_default_max_tokens_when_chat_request_omits_token_limits() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [{"role": "user", "content": "hello"}]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["max_tokens"], 4096);
}
