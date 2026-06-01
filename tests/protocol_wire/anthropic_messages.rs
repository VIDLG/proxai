use proxai::protocol::anthropic::messages::*;
use serde_json::{json, Value};

#[test]
fn deserializes_basic_message_response_from_api_shape() {
    let raw = json!({
        "id": "msg_01XFDUDYJgAACzvnptvVoYEL",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "Hello!"
            }
        ],
        "model": "claude-opus-4-1-20250805",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 12,
            "output_tokens": 6
        }
    });

    let message: Message = serde_json::from_value(raw).expect("deserialize message response");

    assert_eq!(message.id, "msg_01XFDUDYJgAACzvnptvVoYEL");
    assert_eq!(message.type_, MessageType::Message);
    assert_eq!(message.role, Role::Assistant);
    assert_eq!(message.stop_reason, Some(StopReason::EndTurn));
    assert_eq!(message.usage.input_tokens, 12);
    assert_eq!(message.usage.output_tokens, 6);
    assert_eq!(message.content.len(), 1);
    assert!(matches!(message.content[0], ContentBlock::Text(_)));
}

#[test]
fn deserializes_and_serializes_basic_message_request() {
    let raw = json!({
        "model": "claude-opus-4-1-20250805",
        "max_tokens": 1024,
        "messages": [
            {"role": "user", "content": "Hello, Claude"}
        ]
    });

    let request: MessageCreateParamsNonStreaming =
        serde_json::from_value(raw).expect("deserialize message request");
    let serialized = serde_json::to_value(request).expect("serialize message request");

    assert_eq!(serialized["model"], "claude-opus-4-1-20250805");
    assert_eq!(serialized["max_tokens"], 1024);
    assert_eq!(serialized["messages"][0]["role"], "user");
    assert_eq!(serialized["messages"][0]["content"], "Hello, Claude");
    assert!(serialized.get("stream").is_none());
    assert!(serialized.get("thinking").is_none());
    assert!(serialized.get("tools").is_none());
}

#[test]
fn serializes_request_with_text_blocks_thinking_and_custom_tool() {
    let raw = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 256,
        "system": [
            {"type": "text", "text": "Be concise.", "cache_control": {"type": "ephemeral", "ttl": "5m"}}
        ],
        "thinking": {"type": "enabled", "budget_tokens": 128},
        "tool_choice": {"type": "auto", "disable_parallel_tool_use": true},
        "tools": [{
            "type": "custom",
            "name": "get_weather",
            "description": "Get weather for a city.",
            "input_schema": {
                "type": "object",
                "properties": {"city": {"type": "string"}},
                "required": ["city"]
            }
        }],
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "Weather in London?"}
            ]
        }]
    });

    let request: MessageCreateParamsNonStreaming =
        serde_json::from_value(raw).expect("deserialize rich message request");
    let serialized = serde_json::to_value(request).expect("serialize rich message request");

    assert_eq!(serialized["system"][0]["type"], "text");
    assert_eq!(serialized["system"][0]["cache_control"]["ttl"], "5m");
    assert_eq!(serialized["thinking"]["type"], "enabled");
    assert_eq!(serialized["tool_choice"]["type"], "auto");
    assert_eq!(serialized["tools"][0]["type"], "custom");
    assert_eq!(
        serialized["messages"][0]["content"][0]["text"],
        "Weather in London?"
    );
}

#[test]
fn deserializes_common_stream_events_from_data_lines() {
    let events = [
        json!({
            "type": "message_start",
            "message": {
                "id": "msg_01",
                "type": "message",
                "role": "assistant",
                "content": [],
                "model": "claude-sonnet-4-5",
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {"input_tokens": 10, "output_tokens": 1}
            }
        }),
        json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "text", "text": ""}
        }),
        json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "Hello"}
        }),
        json!({
            "type": "message_delta",
            "delta": {
                "stop_reason": "end_turn",
                "stop_sequence": null
            },
            "usage": {"output_tokens": 6}
        }),
        json!({"type": "message_stop"}),
    ];

    for event in events {
        let parsed: MessageStreamEvent =
            serde_json::from_value(event).expect("deserialize stream event");
        let serialized = serde_json::to_value(parsed).expect("serialize stream event");
        assert!(matches!(serialized, Value::Object(_)));
    }
}
