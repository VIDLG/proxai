use serde_json::json;

use crate::protocol::anthropic::messages::{Message, MessageStreamEvent, MessageType};

use super::{normalize_message_payload, normalize_stream_event_payload};

#[test]
fn normalizes_non_stream_provider_message_required_nullable_fields() {
    let payload = json!({
        "id": "msg_compat",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [
            {
                "type": "thinking",
                "thinking": "plan",
                "signature": "sig"
            },
            {
                "type": "tool_use",
                "id": "toolu_1",
                "caller": {"type": "direct"},
                "name": "lookup",
                "input": {}
            }
        ],
        "stop_reason": "tool_use",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 4,
            "server_tool_use": {"web_search_requests": 1, "web_fetch_requests": 0}
        }
    });

    let normalized = normalize_message_payload(payload);
    assert_eq!(normalized["usage"]["cache_creation"], json!(null));
    assert_eq!(
        normalized["usage"]["cache_creation_input_tokens"],
        json!(null)
    );
    assert_eq!(normalized["usage"]["cache_read_input_tokens"], json!(null));
    assert_eq!(normalized["usage"]["inference_geo"], json!(null));
    assert_eq!(normalized["usage"]["service_tier"], json!(null));

    let message: Message = serde_json::from_value(normalized).unwrap();

    assert_eq!(message.id, "msg_compat");
    assert_eq!(message.type_, MessageType::Message);
    assert_eq!(message.content.len(), 2);
}

#[test]
fn leaves_non_message_json_body_unchanged() {
    let payload = json!({"ok": true});

    assert_eq!(normalize_message_payload(payload.clone()), payload);
}

#[test]
fn normalizes_stream_tool_and_usage_required_nullable_fields() {
    let tool_start = json!({
        "type": "content_block_start",
        "index": 0,
        "content_block": {
            "type": "tool_use",
            "id": "toolu_1",
            "caller": {"type": "direct"},
            "name": "lookup",
            "input": {}
        }
    });
    let event: MessageStreamEvent =
        serde_json::from_value(normalize_stream_event_payload(tool_start)).unwrap();
    let MessageStreamEvent::ContentBlockStart(event) = event else {
        panic!("expected content_block_start");
    };
    assert!(matches!(
        event.content_block,
        crate::protocol::anthropic::messages::ContentBlock::ToolUse(_)
    ));

    let delta = json!({
        "type": "message_delta",
        "delta": {
            "stop_reason": "tool_use",
            "stop_sequence": null
        },
        "usage": {
            "output_tokens": 2,
            "server_tool_use": {"web_search_requests": 1, "web_fetch_requests": 0}
        }
    });
    let normalized = normalize_stream_event_payload(delta);
    assert_eq!(
        normalized["usage"]["cache_creation_input_tokens"],
        json!(null)
    );
    assert_eq!(normalized["usage"]["cache_read_input_tokens"], json!(null));
    assert_eq!(normalized["usage"]["input_tokens"], json!(null));

    let event: MessageStreamEvent = serde_json::from_value(normalized).unwrap();
    let MessageStreamEvent::MessageDelta(_event) = event else {
        panic!("expected message_delta");
    };
}

#[test]
fn normalizes_text_block_required_nullable_fields() {
    let payload = json!({
        "type": "content_block_start",
        "index": 0,
        "content_block": {
            "type": "text",
            "text": "hello"
        }
    });

    let normalized = normalize_stream_event_payload(payload);
    assert_eq!(normalized["content_block"]["citations"], json!(null));

    let event: MessageStreamEvent = serde_json::from_value(normalized).unwrap();

    let MessageStreamEvent::ContentBlockStart(event) = event else {
        panic!("expected content_block_start");
    };
    let crate::protocol::anthropic::messages::ContentBlock::Text(block) = event.content_block
    else {
        panic!("expected text block");
    };
    assert_eq!(block.citations, None);
}

#[test]
fn normalizes_glm5_partial_server_tool_usage_counters() {
    let payload = json!({
        "type": "message_delta",
        "delta": {
            "stop_reason": "end_turn",
            "stop_sequence": null
        },
        "usage": {
            "input_tokens": 165,
            "output_tokens": 8,
            "cache_read_input_tokens": 0,
            "server_tool_use": {"web_search_requests": 0},
            "service_tier": "standard"
        }
    });

    let normalized = normalize_stream_event_payload(payload);
    assert_eq!(
        normalized["usage"]["server_tool_use"]["web_fetch_requests"],
        json!(0)
    );
    assert_eq!(
        normalized["usage"]["server_tool_use"]["web_search_requests"],
        json!(0)
    );

    let event: MessageStreamEvent = serde_json::from_value(normalized).unwrap();
    let MessageStreamEvent::MessageDelta(event) = event else {
        panic!("expected message_delta");
    };
    assert_eq!(
        event
            .usage
            .server_tool_use
            .expect("server tool usage")
            .web_fetch_requests,
        0
    );
}

#[test]
fn normalizes_minimax_thinking_start_without_signature() {
    let payload = json!({
        "type": "content_block_start",
        "index": 0,
        "content_block": {
            "type": "thinking",
            "thinking": ""
        }
    });

    let normalized = normalize_stream_event_payload(payload);
    assert_eq!(normalized["content_block"]["signature"], json!(""));

    let event: MessageStreamEvent = serde_json::from_value(normalized).unwrap();

    let MessageStreamEvent::ContentBlockStart(event) = event else {
        panic!("expected content_block_start");
    };
    let crate::protocol::anthropic::messages::ContentBlock::Thinking(block) = event.content_block
    else {
        panic!("expected thinking block");
    };
    assert_eq!(block.signature, "");
}
