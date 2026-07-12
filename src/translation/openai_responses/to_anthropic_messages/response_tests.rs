use serde_json::json;

use crate::protocol::anthropic::messages::ContentBlock;
use crate::protocol::openai_responses::Response;

use super::translate_response_payload;

#[test]
fn translates_openai_response_to_anthropic_message_shape() {
    let response: Response = serde_json::from_value(json!({
        "id": "resp_123",
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
        "created_at": 0,
        "model": "glm-5.1",
        "status": "completed",
        "output": [
            {
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": "hello", "annotations": [], "logprobs": []}]
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
                "summary": [],
                "content": [{"type": "reasoning_text", "text": "thinking"}],
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
    assert_eq!(serialized["stop_reason"], "tool_use");
    assert_eq!(serialized["usage"]["input_tokens"], 10);
    assert_eq!(serialized["usage"]["cache_read_input_tokens"], 2);
    assert_eq!(
        serialized["usage"]["output_tokens_details"]["thinking_tokens"],
        1
    );
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

#[test]
fn maps_failed_responses_status_without_refusal_to_missing_anthropic_stop_reason() {
    let response: Response = serde_json::from_value(json!({
        "id": "resp_failed",
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
        "created_at": 0,
        "model": "glm-5.1",
        "status": "failed",
        "error": {"type": "server_error", "code": "upstream_failed", "message": "boom"},
        "output": [{
            "type": "message",
            "id": "msg_1",
            "role": "assistant",
            "status": "completed",
            "content": [{"type": "output_text", "text": "partial", "annotations": [], "logprobs": []}]
        }]
    }))
    .unwrap();

    let translated = translate_response_payload(&response);
    let serialized = serde_json::to_value(&translated).unwrap();

    assert!(serialized["stop_reason"].is_null());
}

#[test]
fn rejects_unknown_responses_incomplete_reason_at_protocol_boundary() {
    let error = serde_json::from_value::<Response>(json!({
        "id": "resp_unknown_incomplete",
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
        "created_at": 0,
        "model": "glm-5.1",
        "status": "incomplete",
        "incomplete_details": {"reason": "provider_shutdown"},
        "output": [{
            "type": "message",
            "id": "msg_1",
            "role": "assistant",
            "status": "incomplete",
            "content": [{"type": "output_text", "text": "partial", "annotations": [], "logprobs": []}]
        }]
    }))
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("unknown variant `provider_shutdown`")
    );
}

#[test]
fn maps_responses_refusal_content_to_anthropic_refusal_stop_reason() {
    let response: Response = serde_json::from_value(json!({
        "id": "resp_refusal",
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
        "created_at": 0,
        "model": "glm-5.1",
        "status": "completed",
        "output": [{
            "type": "message",
            "id": "msg_1",
            "role": "assistant",
            "status": "completed",
            "content": [{"type": "refusal", "refusal": "I can't help with that."}]
        }]
    }))
    .unwrap();

    let translated = translate_response_payload(&response);
    let serialized = serde_json::to_value(&translated).unwrap();

    assert_eq!(serialized["stop_reason"], "refusal");
}
