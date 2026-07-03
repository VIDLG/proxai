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
fn translates_glm_responses_request_with_item_reference_placeholder() {
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

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["model"], "glm-5.1");
    assert_eq!(translated["max_tokens"], 128);
    assert_eq!(translated["system"], "Be concise.");
    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(translated["messages"][0]["content"][0]["text"], "hello");
    assert_eq!(translated["messages"][1]["role"], "user");
    assert_eq!(
        translated["messages"][1]["content"],
        "[OpenAI Responses item_reference `future_zed_item` omitted during Anthropic translation]"
    );
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
fn translates_minimal_reasoning_effort_to_disabled_thinking() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": "hello",
        "reasoning": {"effort": "minimal", "summary": "auto"}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert!(translated.get("output_config").is_none());
    assert_eq!(translated["thinking"]["type"], "disabled");
    assert!(translated["thinking"].get("display").is_none());
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
            {"type": "custom_tool_call", "call_id": "call_1", "name": "shell", "input": "pwd"},
            {"type": "custom_tool_call", "call_id": "call_2", "name": "shell", "input": "ls"},
            {"type": "custom_tool_call_output", "call_id": "call_1", "output": "D:/projects/proxai"},
            {"type": "custom_tool_call_output", "call_id": "call_2", "output": ["Cargo.toml", "src"]}
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();
    let messages = translated["messages"].as_array().unwrap();

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[1]["content"].as_array().unwrap().len(), 2);
    assert_eq!(messages[2]["role"], "user");
    assert_eq!(messages[2]["content"].as_array().unwrap().len(), 2);
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
