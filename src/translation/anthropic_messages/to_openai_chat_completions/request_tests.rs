use serde_json::json;

use super::translate_request_payload;

#[test]
fn translates_anthropic_request_to_chat_completion_shape() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "stream": true,
        "system": [{"type": "text", "text": "You are concise."}],
        "messages": [
            {
                "role": "user",
                "content": [{"type": "text", "text": "Call the tool."}]
            },
            {
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Sure."},
                    {"type": "tool_use", "id": "toolu_1", "name": "lookup", "input": {"query": "proxai"}}
                ]
            },
            {
                "role": "user",
                "content": [{"type": "tool_result", "tool_use_id": "toolu_1", "content": "found"}]
            }
        ],
        "tools": [{
            "type": "custom",
            "name": "lookup",
            "description": "Lookup a value",
            "input_schema": {
                "type": "object",
                "properties": {"query": {"type": "string"}},
                "required": ["query"]
            }
        }],
        "tool_choice": {"type": "tool", "name": "lookup", "disable_parallel_tool_use": true},
        "temperature": 0.2,
        "top_p": 0.9,
        "stop_sequences": ["END"]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["model"], "claude-sonnet-4-5");
    assert_eq!(translated["max_completion_tokens"], 128);
    assert_eq!(translated["stream"], true);
    assert_eq!(translated["messages"][0]["role"], "system");
    assert_eq!(translated["messages"][0]["content"], "You are concise.");
    assert_eq!(translated["messages"][1]["role"], "user");
    assert_eq!(translated["messages"][1]["content"], "Call the tool.");
    assert_eq!(translated["messages"][2]["role"], "assistant");
    assert_eq!(translated["messages"][2]["content"], "Sure.");
    assert_eq!(
        translated["messages"][2]["tool_calls"][0],
        json!({
            "id": "toolu_1",
            "type": "function",
            "function": {"name": "lookup", "arguments": "{\"query\":\"proxai\"}"}
        })
    );
    assert_eq!(translated["messages"][3]["role"], "tool");
    assert_eq!(translated["messages"][3]["tool_call_id"], "toolu_1");
    assert_eq!(translated["messages"][3]["content"], "found");
    assert_eq!(translated["tools"][0]["type"], "function");
    assert_eq!(translated["tools"][0]["function"]["name"], "lookup");
    assert_eq!(
        translated["tools"][0]["function"]["parameters"],
        json!({
            "type": "object",
            "properties": {"query": {"type": "string"}},
            "required": ["query"]
        })
    );
    assert_eq!(
        translated["tool_choice"],
        json!({"function": {"name": "lookup"}})
    );
    assert_eq!(translated["parallel_tool_calls"], false);
    assert_eq!(translated["stop"], "END");
}

#[test]
fn splits_mixed_anthropic_user_content_and_tool_result_into_chat_messages() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "Here is context."},
                {"type": "tool_result", "tool_use_id": "toolu_1", "content": "found"}
            ]
        }]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(translated["messages"][0]["content"], "Here is context.");
    assert_eq!(translated["messages"][1]["role"], "tool");
    assert_eq!(translated["messages"][1]["tool_call_id"], "toolu_1");
    assert_eq!(translated["messages"][1]["content"], "found");
}

#[test]
fn translates_anthropic_tool_result_text_blocks_to_chat_tool_message_array() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": "toolu_1",
                "content": [
                    {"type": "text", "text": "found"},
                    {"type": "text", "text": " it"}
                ]
            }]
        }]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["messages"][0]["role"], "tool");
    assert_eq!(translated["messages"][0]["tool_call_id"], "toolu_1");
    assert_eq!(
        translated["messages"][0]["content"],
        json!([
            {"type": "text", "text": "found"},
            {"type": "text", "text": " it"}
        ])
    );
}

#[test]
fn rejects_non_text_anthropic_tool_result_blocks_for_chat_completion() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": "toolu_1",
                "content": [{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/png",
                        "data": "iVBORw0KGgo="
                    }
                }]
            }]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("tool_result content block `image`"));
}

#[test]
fn rejects_anthropic_assistant_text_after_tool_use_for_chat_completion() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "assistant",
            "content": [
                {"type": "text", "text": "Before."},
                {
                    "type": "tool_use",
                    "id": "toolu_1",
                    "name": "lookup",
                    "input": {"query": "proxai"}
                },
                {"type": "text", "text": "After."}
            ]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("text blocks after tool_use blocks"));
}

#[test]
fn rejects_empty_anthropic_user_content_for_chat_completion() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{"role": "user", "content": ""}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("user message without content"));
}

#[test]
fn rejects_empty_anthropic_assistant_content_for_chat_completion() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{"role": "assistant", "content": []}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("assistant message without content or tool_use"));
}

#[test]
fn translates_anthropic_output_effort_to_chat_reasoning_effort() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{"role": "user", "content": "think"}],
        "output_config": {"effort": "xhigh"},
        "thinking": {"type": "enabled", "budget_tokens": 2048}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["reasoning_effort"], "xhigh");
}

#[test]
fn translates_anthropic_thinking_to_chat_reasoning_effort() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{"role": "user", "content": "think"}],
        "thinking": {"type": "enabled", "budget_tokens": 9000}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["reasoning_effort"], "high");
}

#[test]
fn translates_anthropic_container_upload_to_chat_file_part() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "user",
            "content": [{"type": "container_upload", "file_id": "file_123"}]
        }]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(
        translated["messages"][0]["content"],
        json!([{
            "type": "file",
            "file": {"file_data": null, "file_id": "file_123", "filename": null}
        }])
    );
}

#[test]
fn rejects_unsupported_anthropic_tool_for_chat_completion() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{"role": "user", "content": "search"}],
        "tools": [{"type": "web_search_20250305"}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("Anthropic tool `web_search_20250305`"));
}

#[test]
fn rejects_anthropic_tool_choice_for_missing_chat_tool() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{"role": "user", "content": "Call the tool."}],
        "tools": [{
            "type": "custom",
            "name": "lookup",
            "input_schema": {"type": "object"}
        }],
        "tool_choice": {"type": "tool", "name": "missing"}
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("tool_choice references tool `missing`"));
}

#[test]
fn translates_anthropic_base64_document_to_chat_file_part() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "document",
                "title": "spec.pdf",
                "source": {
                    "type": "base64",
                    "media_type": "application/pdf",
                    "data": "JVBERi0x"
                }
            }]
        }]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(
        translated["messages"][0]["content"][0],
        json!({
            "type": "file",
            "file": {
                "file_data": "JVBERi0x",
                "file_id": null,
                "filename": "spec.pdf"
            }
        })
    );
}

#[test]
fn rejects_unsupported_anthropic_user_block_for_chat_completion() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "thinking",
                "thinking": "hidden chain of thought",
                "signature": "sig"
            }]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("user content block `thinking`"));
}

#[test]
fn rejects_anthropic_request_without_messages_for_chat_completion() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": []
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();
    assert!(error.contains("at least one user or assistant message"));
}
