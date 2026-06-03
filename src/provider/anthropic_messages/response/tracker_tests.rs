use bytes::Bytes;
use serde_json::json;

use super::AnthropicResponseTracker;

#[test]
fn tracker_extracts_stream_message_events() {
    let mut tracker = AnthropicResponseTracker::new();

    let chunk = Bytes::from(format!(
        "data: {}\n\ndata: {}\n\ndata: {}\n\ndata: {}\n\n",
        json!({
            "type": "message_start",
            "message": {
                "id": "msg_stream",
                "type": "message",
                "role": "assistant",
                "model": "claude-test",
                "content": [],
                "stop_reason": null,
                "stop_sequence": null,
                "stop_details": null,
                "container": null,
                "usage": {
                    "input_tokens": 8,
                    "output_tokens": 0,
                    "cache_creation": null,
                    "cache_creation_input_tokens": null,
                    "cache_read_input_tokens": null,
                    "inference_geo": null,
                    "server_tool_use": null,
                    "service_tier": "priority"
                }
            }
        }),
        json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "ok"}
        }),
        json!({
            "type": "message_delta",
            "delta": {
                "stop_reason": "end_turn",
                "stop_sequence": null,
                "stop_details": null,
                "container": null
            },
            "usage": {
                "input_tokens": 8,
                "output_tokens": 2,
                "cache_creation_input_tokens": null,
                "cache_read_input_tokens": null,
                "server_tool_use": {"web_search_requests": 1, "web_fetch_requests": 0}
            }
        }),
        json!({"type": "message_stop"})
    ));

    tracker.scan_bytes(&chunk);

    let state = &tracker.state;
    assert_eq!(state.id().as_deref(), Some("msg_stream"));
    assert_eq!(state.model().as_deref(), Some("claude-test"));
    assert!(tracker.state.stream_done());
    assert_eq!(state.output_tokens(), Some(2));

    assert_eq!(tracker.state.summary.stop_reasons.get("end_turn"), Some(&1));
    assert_eq!(
        tracker.state.summary.server_tool_uses.get("web_search"),
        Some(&1)
    );
}

#[test]
fn tracker_normalizes_provider_stream_event_before_observing() {
    let mut tracker = AnthropicResponseTracker::new();

    let chunk = Bytes::from(format!(
        "data: {}\n\ndata: {}\n\ndata: {}\n\n",
        json!({
            "type": "message_start",
            "message": {
                "id": "msg_stream",
                "type": "message",
                "role": "assistant",
                "model": "glm-5.1",
                "content": [],
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {
                    "input_tokens": 8,
                    "output_tokens": 0
                }
            }
        }),
        json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "tool_use", "id": "toolu_1", "caller": {"type": "direct"}, "name": "lookup", "input": {}}
        }),
        json!({
            "type": "message_delta",
            "delta": {"stop_reason": "tool_use", "stop_sequence": null},
            "usage": {
                "output_tokens": 2,
                "server_tool_use": {"web_search_requests": 1, "web_fetch_requests": 0}
            }
        })
    ));

    tracker.scan_bytes(&chunk);

    let state = &tracker.state;
    assert_eq!(state.id().as_deref(), Some("msg_stream"));
    assert_eq!(tracker.state.summary.tool_uses.get("lookup"), Some(&1));
    assert_eq!(
        tracker.state.summary.server_tool_uses.get("web_search"),
        Some(&1)
    );
}
