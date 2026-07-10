use serde_json::{Value, json};

use crate::translation::openai_responses::to_anthropic_messages::translate_request_payload;

#[test]
fn translates_text_request_with_instructions_and_function_tool() {
    let payload = json!({
        "model": "gpt-5.5",
        "instructions": "Be concise.",
        "input": "hello",
        "max_output_tokens": 123,
        "stream": true,
        "parallel_tool_calls": false,
        "tool_choice": "required",
        "tools": [{
            "type": "function",
            "name": "lookup",
            "description": "Look up a record",
            "parameters": {
                "type": "object",
                "properties": {"id": {"type": "string"}},
                "required": ["id"]
            }
        }],
        "reasoning": {"effort": "high", "summary": "auto"}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["model"], "gpt-5.5");
    assert_eq!(translated["max_tokens"], 123);
    assert_eq!(translated["system"], "Be concise.");
    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(translated["messages"][0]["content"], "hello");
    assert_eq!(translated["tools"][0]["type"], "custom");
    assert_eq!(translated["tools"][0]["name"], "lookup");
    assert_eq!(translated["tool_choice"]["type"], "any");
    assert_eq!(translated["tool_choice"]["disable_parallel_tool_use"], true);
    assert_eq!(translated["output_config"]["effort"], "high");
    assert_eq!(translated["thinking"]["type"], "adaptive");
    assert_eq!(translated["thinking"]["display"], "summarized");
}

#[test]
fn reports_json_location_for_invalid_responses_request_payload() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": {"unexpected": true}
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("failed to deserialize normalized translation payload"));
    assert!(error.contains("OpenAI Responses request payload"));
    assert!(error.contains("JSON path `input`"));
    assert!(error.contains("pretty line "));
    assert!(error.contains("column "));
}

#[test]
fn rejects_responses_request_with_item_reference() {
    let payload = json!({
        "model": "glm-5.1",
        "instructions": "Be concise.",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "hello"}]
            },
            {
                "type": "item_reference",
                "id": "future_zed_item"
            }
        ],
        "max_output_tokens": 128,
        "stream": true
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("item_reference `future_zed_item` cannot be translated"));
}

#[test]
fn rejects_system_or_developer_content_list_with_non_text_parts() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [
            {
                "type": "message",
                "role": "developer",
                "content": [{"type": "input_image", "image_url": "https://example.test/policy.png"}]
            },
            {
                "type": "message",
                "role": "user",
                "content": "hello"
            }
        ]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("system/developer message content cannot include input_image"));
}

#[test]
fn skips_unsigned_responses_reasoning_history() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": "hello"
            },
            {
                "type": "reasoning",
                "id": "rs_123",
                "summary": [{"type": "summary_text", "text": "internal reasoning summary"}],
                "content": [{"type": "reasoning_text", "text": "private chain of thought"}]
            },
            {
                "type": "message",
                "id": "msg_123",
                "status": "completed",
                "role": "assistant",
                "content": [{
                    "type": "output_text",
                    "text": "Hi!",
                    "annotations": []
                }]
            }
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["messages"].as_array().unwrap().len(), 2);
    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(translated["messages"][0]["content"], "hello");
    assert_eq!(translated["messages"][1]["role"], "assistant");
    assert_eq!(translated["messages"][1]["content"][0]["type"], "text");
    assert_eq!(translated["messages"][1]["content"][0]["text"], "Hi!");
    assert!(!translated.to_string().contains("private chain of thought"));
}

#[test]
fn preserves_responses_encrypted_reasoning_as_anthropic_redacted_thinking() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [
            {"type": "message", "role": "user", "content": "continue"},
            {
                "type": "reasoning",
                "id": "rs_redacted",
                "summary": [],
                "encrypted_content": "anthropic-redacted-thinking-data"
            }
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["messages"].as_array().unwrap().len(), 2);
    assert_eq!(translated["messages"][1]["role"], "assistant");
    assert_eq!(
        translated["messages"][1]["content"][0]["type"],
        "redacted_thinking"
    );
    assert_eq!(
        translated["messages"][1]["content"][0]["data"],
        "anthropic-redacted-thinking-data"
    );
}

#[test]
fn rejects_unsupported_responses_tools_for_anthropic_translation() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": "hello",
        "tools": [{
            "type": "file_search",
            "vector_store_ids": ["vs_123"]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("tool `file_search` cannot be translated"));
}

#[test]
fn rejects_unsupported_responses_tool_choice_for_anthropic_translation() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": "hello",
        "tool_choice": {"type": "apply_patch"}
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("tool_choice `apply_patch` cannot be translated"));
}

#[test]
fn rejects_responses_request_without_anthropic_messages() {
    let payload = json!({
        "model": "gpt-5.5",
        "instructions": "Be concise.",
        "input": [
            {"type": "message", "role": "developer", "content": "Follow policy."},
            {"type": "message", "role": "system", "content": "System policy."}
        ]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("at least one user or assistant input item"));
}

#[test]
fn translates_reasoning_effort_without_summary_to_output_config_only() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": "hello",
        "reasoning": {"effort": "high"}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["output_config"]["effort"], "high");
    assert!(translated.get("thinking").is_none());
}

#[test]
fn translates_reasoning_summary_without_effort_to_adaptive_thinking_display() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": "hello",
        "reasoning": {"summary": "detailed"}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert!(translated.get("output_config").is_none());
    assert_eq!(translated["thinking"]["type"], "adaptive");
    assert_eq!(translated["thinking"]["display"], "summarized");
}

#[test]
fn translates_reasoning_effort_and_summary_to_output_config_plus_display() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": "hello",
        "reasoning": {"effort": "medium", "summary": "concise"}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["output_config"]["effort"], "medium");
    assert_eq!(translated["thinking"]["type"], "adaptive");
    assert_eq!(translated["thinking"]["display"], "summarized");
}

#[test]
fn rejects_minimal_reasoning_effort_for_anthropic_output_config() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": "hello",
        "reasoning": {"effort": "minimal", "summary": "auto"}
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("reasoning effort `minimal` cannot be translated"));
    assert!(error.contains("output_config.effort"));
}

#[test]
fn translates_responses_image_and_file_content_to_anthropic_blocks() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [
                {"type": "input_text", "text": "Review these assets"},
                {"type": "input_image", "image_url": "data:image/png;base64,iVBORw0KGgo="},
                {"type": "input_file", "filename": "spec.pdf", "file_data": "data:application/pdf;base64,JVBERi0x"},
                {"type": "input_file", "filename": "notes.txt", "file_data": "plain notes"}
            ]
        }]
    });

    let translated = translate_request_payload(&payload).unwrap();
    let content = translated["messages"][0]["content"].as_array().unwrap();

    assert_eq!(content[1]["type"], "image");
    assert_eq!(content[1]["source"]["type"], "base64");
    assert_eq!(content[1]["source"]["media_type"], "image/png");
    assert_eq!(content[1]["source"]["data"], "iVBORw0KGgo=");
    assert_eq!(content[2]["type"], "document");
    assert_eq!(content[2]["title"], "spec.pdf");
    assert_eq!(content[2]["source"]["type"], "base64");
    assert_eq!(content[2]["source"]["media_type"], "application/pdf");
    assert_eq!(content[2]["source"]["data"], "JVBERi0x");
    assert_eq!(content[3]["type"], "document");
    assert_eq!(content[3]["title"], "notes.txt");
    assert_eq!(content[3]["source"]["type"], "text");
    assert_eq!(content[3]["source"]["media_type"], "text/plain");
    assert_eq!(content[3]["source"]["data"], "plain notes");
}

#[test]
fn rejects_bare_base64_responses_image_content_for_anthropic_translation() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{"type": "input_image", "image_url": "iVBORw0KGgo="}]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("http(s) URL or data:image"));
}

#[test]
fn rejects_non_pdf_responses_file_urls_for_anthropic_documents() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_file",
                "filename": "notes.txt",
                "file_url": "https://example.test/notes.txt"
            }]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("Document URL values must be an http(s) PDF URL"));
}

#[test]
fn rejects_provider_scoped_responses_file_ids_for_anthropic_translation() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{"type": "input_file", "file_id": "file_123"}]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("input_file.file_id cannot be translated"));
    assert!(error.contains("provider-scoped"));
}

#[test]
fn translates_message_items_and_tool_roundtrip_items() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [
            {"type": "message", "role": "developer", "content": "Follow policy."},
            {"type": "message", "role": "user", "content": [
                {"type": "input_text", "text": "Look this up"},
                {"type": "input_image", "image_url": "https://example.test/a.png"}
            ]},
            {"type": "function_call", "call_id": "call_1", "name": "lookup", "arguments": "{\"id\":\"42\"}"},
            {"type": "function_call_output", "call_id": "call_1", "output": "result"}
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["max_tokens"], 4096);
    assert_eq!(translated["system"], "Follow policy.");
    assert_eq!(
        translated["messages"][0]["content"][0]["text"],
        "Look this up"
    );
    assert_eq!(translated["messages"][0]["content"][1]["type"], "image");
    assert_eq!(translated["messages"][1]["role"], "assistant");
    assert_eq!(translated["messages"][1]["content"][0]["type"], "tool_use");
    assert_eq!(translated["messages"][1]["content"][0]["input"]["id"], "42");
    assert_eq!(
        translated["messages"][2]["content"][0]["type"],
        "tool_result"
    );
}

#[test]
fn groups_parallel_tool_calls_and_results_into_adjacent_messages() {
    let payload = json!({
        "model": "MiniMax-M3",
        "input": [
            {"type": "message", "role": "user", "content": "Use both tools"},
            {"type": "function_call", "call_id": "call_1", "name": "lookup", "arguments": "{\"id\":\"42\"}"},
            {"type": "function_call", "call_id": "call_2", "name": "search", "arguments": "{\"q\":\"proxai\"}"},
            {"type": "function_call_output", "call_id": "call_1", "output": "lookup result"},
            {"type": "function_call_output", "call_id": "call_2", "output": "search result"},
            {"type": "message", "role": "user", "content": "Continue"}
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();
    let messages = translated["messages"].as_array().unwrap();

    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[1]["content"].as_array().unwrap().len(), 2);
    assert_eq!(messages[1]["content"][0]["type"], "tool_use");
    assert_eq!(messages[1]["content"][0]["id"], "call_1");
    assert_eq!(messages[1]["content"][1]["type"], "tool_use");
    assert_eq!(messages[1]["content"][1]["id"], "call_2");
    assert_eq!(messages[2]["role"], "user");
    assert_eq!(messages[2]["content"].as_array().unwrap().len(), 2);
    assert_eq!(messages[2]["content"][0]["type"], "tool_result");
    assert_eq!(messages[2]["content"][0]["tool_use_id"], "call_1");
    assert_eq!(messages[2]["content"][1]["type"], "tool_result");
    assert_eq!(messages[2]["content"][1]["tool_use_id"], "call_2");
    assert_eq!(messages[3]["role"], "user");
    assert_tool_results_immediately_follow_tool_uses(messages);
}

#[test]
fn groups_custom_tool_calls_and_results_into_adjacent_messages() {
    let payload = json!({
        "model": "MiniMax-M3",
        "input": [
            {"type": "message", "role": "user", "content": "Use custom tools"},
            {"type": "custom_tool_call", "id": "item_1", "call_id": "call_1", "name": "shell", "input": "pwd"},
            {"type": "custom_tool_call", "id": "item_2", "call_id": "call_2", "name": "shell", "input": "ls"},
            {"type": "custom_tool_call_output", "call_id": "call_1", "output": "D:/projects/proxai"},
            {
                "type": "custom_tool_call_output",
                "call_id": "call_2",
                "output": [
                    {"type": "input_text", "text": "Cargo.toml"},
                    {"type": "input_text", "text": "src"}
                ]
            }
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();
    let messages = translated["messages"].as_array().unwrap();

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[1]["content"].as_array().unwrap().len(), 2);
    assert_eq!(messages[2]["role"], "user");
    assert_eq!(messages[2]["content"].as_array().unwrap().len(), 2);
    assert_eq!(
        messages[2]["content"][1]["content"][0]["text"],
        "Cargo.toml"
    );
    assert_eq!(messages[2]["content"][1]["content"][1]["text"], "src");
    assert_tool_results_immediately_follow_tool_uses(messages);
}

#[test]
fn translates_glm_responses_request_to_anthropic_messages_shape() {
    let payload = json!({
        "model": "glm-5.1",
        "instructions": "You are a proxai live translation smoke test. Reply briefly.",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "Reply with the exact text: proxai-translation-live-ok"
            }]
        }],
        "stream": false,
        "max_output_tokens": 64
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["model"], "glm-5.1");
    assert_eq!(translated["max_tokens"], 64);
    assert_eq!(
        translated["system"],
        "You are a proxai live translation smoke test. Reply briefly."
    );
    assert_eq!(translated["stream"], false);
    assert_eq!(translated["messages"].as_array().unwrap().len(), 1);
    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(
        translated["messages"][0]["content"][0],
        json!({
            "type": "text",
            "text": "Reply with the exact text: proxai-translation-live-ok"
        })
    );
}

fn assert_tool_results_immediately_follow_tool_uses(messages: &[Value]) {
    for (index, message) in messages.iter().enumerate() {
        let content = message
            .get("content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let tool_use_ids = content
            .iter()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some("tool_use"))
            .filter_map(|block| block.get("id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        if tool_use_ids.is_empty() {
            continue;
        }

        assert_eq!(message["role"], "assistant");
        let next = messages.get(index + 1).unwrap_or_else(|| {
            panic!("tool_use message at index {index} has no following message")
        });
        assert_eq!(next["role"], "user");
        let result_ids = next
            .get("content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some("tool_result"))
            .filter_map(|block| block.get("tool_use_id").and_then(Value::as_str))
            .collect::<Vec<_>>();

        assert_eq!(result_ids, tool_use_ids);
    }
}
