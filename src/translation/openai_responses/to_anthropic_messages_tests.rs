use serde_json::{json, Value};

use crate::protocol::anthropic::messages::ContentBlock;

use axum::body::{to_bytes, Body};
use axum::http::{header, Response};

use super::{translate_request_payload, translate_response_payload, OpenaiResponseBody};

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
        }]
    });

    let translated = translate_request_payload(&payload, "gpt-5.5", "claude-sonnet").unwrap();

    assert_eq!(translated["model"], "claude-sonnet");
    assert_eq!(translated["max_tokens"], 123);
    assert_eq!(translated["system"], "Be concise.");
    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(translated["messages"][0]["content"], "hello");
    assert_eq!(translated["tools"][0]["type"], "custom");
    assert_eq!(translated["tools"][0]["name"], "lookup");
    assert_eq!(translated["tool_choice"]["type"], "any");
    assert_eq!(translated["tool_choice"]["disable_parallel_tool_use"], true);
}

#[test]
fn translates_glm_responses_request_with_unknown_input_item() {
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
                "type": "future_zed_item",
                "opaque": {"value": 1}
            }
        ],
        "max_output_tokens": 128,
        "stream": true
    });

    let translated = translate_request_payload(&payload, "glm-5.1", "glm-5.1").unwrap();

    assert_eq!(translated["model"], "glm-5.1");
    assert_eq!(translated["max_tokens"], 128);
    assert_eq!(translated["system"], "Be concise.");
    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(translated["messages"][0]["content"][0]["text"], "hello");
    assert_eq!(translated["messages"][1]["role"], "user");
    assert_eq!(
        translated["messages"][1]["content"],
        "[OpenAI Responses item `future_zed_item` omitted during Anthropic translation]"
    );
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

    let translated = translate_request_payload(&payload, "gpt-5.5", "claude-sonnet").unwrap();

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

    let translated = translate_request_payload(&payload, "MiniMax-M3", "MiniMax-M3").unwrap();
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

    let translated = translate_request_payload(&payload, "MiniMax-M3", "MiniMax-M3").unwrap();
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

    let translated = translate_request_payload(&payload, "glm-5.1", "glm-5.1").unwrap();

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

#[test]
fn translates_openai_response_to_anthropic_message_shape() {
    let response: OpenaiResponseBody = serde_json::from_value(json!({
        "id": "resp_123",
        "object": "response",
        "created_at": 0,
        "model": "glm-5.1",
        "status": "completed",
        "output": [
            {
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": "hello", "annotations": []}]
            },
            {
                "type": "function_call",
                "id": "fc_1",
                "call_id": "call_1",
                "name": "lookup",
                "arguments": "{\"id\":\"42\"}",
                "status": "completed"
            },
            {
                "type": "reasoning",
                "id": "rs_1",
                "summary": [{"type": "summary_text", "text": "thinking"}],
                "status": "completed"
            }
        ],
        "usage": {
            "input_tokens": 10,
            "input_tokens_details": {"cached_tokens": 2},
            "output_tokens": 6,
            "output_tokens_details": {"reasoning_tokens": 1},
            "total_tokens": 16
        }
    }))
    .unwrap();

    let translated = translate_response_payload(&response);
    let serialized = serde_json::to_value(&translated).unwrap();

    assert_eq!(serialized["id"], "resp_123");
    assert_eq!(serialized["type"], "message");
    assert_eq!(serialized["role"], "assistant");
    assert_eq!(serialized["model"], "glm-5.1");
    assert_eq!(serialized["stop_reason"], "end_turn");
    assert_eq!(serialized["usage"]["input_tokens"], 10);
    assert_eq!(serialized["usage"]["cache_read_input_tokens"], 2);
    assert_eq!(serialized["content"][0]["type"], "text");
    assert_eq!(serialized["content"][0]["text"], "hello");
    assert_eq!(serialized["content"][1]["type"], "tool_use");
    assert_eq!(serialized["content"][1]["id"], "call_1");
    assert_eq!(serialized["content"][1]["input"]["id"], "42");
    assert!(matches!(
        translated.content.get(2),
        Some(ContentBlock::Thinking(block)) if block.thinking == "thinking"
    ));
}

#[tokio::test]
async fn translates_openai_responses_stream_to_anthropic_messages_sse() {
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(Body::from(
            "event: response.created\n\
data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_123\",\"model\":\"glm-5.1\",\"usage\":{\"input_tokens\":8,\"input_tokens_details\":{\"cached_tokens\":0},\"output_tokens\":0,\"output_tokens_details\":{\"reasoning_tokens\":0},\"total_tokens\":8}}}\n\n\
event: response.output_item.added\n\
data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n\
event: response.output_text.delta\n\
data: {\"type\":\"response.output_text.delta\",\"sequence_number\":3,\"output_index\":0,\"content_index\":0,\"item_id\":\"msg_1\",\"delta\":\"ok\"}\n\n\
event: response.output_text.done\n\
data: {\"type\":\"response.output_text.done\",\"sequence_number\":4,\"output_index\":0,\"content_index\":0,\"item_id\":\"msg_1\",\"text\":\"ok\"}\n\n\
event: response.completed\n\
data: {\"type\":\"response.completed\",\"sequence_number\":5,\"response\":{\"id\":\"resp_123\",\"model\":\"glm-5.1\",\"usage\":{\"input_tokens\":8,\"input_tokens_details\":{\"cached_tokens\":0},\"output_tokens\":2,\"output_tokens_details\":{\"reasoning_tokens\":0},\"total_tokens\":10}}}\n\n",
        ))
        .unwrap();

    let translated = super::translate_response(response).await.unwrap();
    let body = to_bytes(translated.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("event: message_start"));
    assert!(body.contains("\"id\":\"resp_123\""));
    assert!(body.contains("event: content_block_delta"));
    assert!(body.contains("\"type\":\"text_delta\""));
    assert!(body.contains("\"text\":\"ok\""));
    assert!(body.contains("event: message_delta"));
    assert!(body.contains("\"stop_reason\":\"end_turn\""));
    assert!(body.contains("event: message_stop"));
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
