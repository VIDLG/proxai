use axum::http::HeaderMap;
use bytes::Bytes;
use serde_json::json;

use super::AnthropicResponseTracker;

use crate::provider::anthropic_messages::summary::AnthropicResponseOutputKind;

#[test]
fn tracker_extracts_non_stream_message_usage_and_summary() {
    let headers = HeaderMap::new();
    let mut tracker = AnthropicResponseTracker::from_headers(&headers);
    let body = json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "model": "claude-test",
        "content": [
            {"type": "text", "text": "ok", "citations": null},
            {"type": "tool_use", "id": "toolu_1", "name": "lookup", "input": {}, "caller": {"type": "direct"}}
        ],
        "stop_reason": "tool_use",
        "stop_sequence": null,
        "stop_details": null,
        "container": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 4,
            "cache_creation": null,
            "cache_creation_input_tokens": 2,
            "cache_read_input_tokens": 3,
            "inference_geo": null,
            "server_tool_use": null,
            "service_tier": "standard"
        }
    });

    let bytes = serde_json::to_vec(&body).unwrap();
    tracker.scan_bytes(&bytes[..8]);
    tracker.scan_bytes(&bytes[8..]);
    tracker.finish();

    let projection = &tracker.state.projection;
    assert_eq!(projection.id().as_deref(), Some("msg_123"));
    assert_eq!(projection.model().as_deref(), Some("claude-test"));
    assert_eq!(projection.input_tokens(), Some(10));
    assert_eq!(projection.output_tokens(), Some(4));

    assert_eq!(
        tracker
            .state
            .summary
            .output_items
            .get(&AnthropicResponseOutputKind::Text),
        Some(&1)
    );
    assert_eq!(tracker.state.summary.stop_reasons.get("tool_use"), Some(&1));
    assert_eq!(tracker.state.summary.tool_uses.get("lookup"), Some(&1));
}

#[test]
fn tracker_normalizes_provider_message_before_observing() {
    let headers = HeaderMap::new();
    let mut tracker = AnthropicResponseTracker::from_headers(&headers);
    let body = json!({
        "id": "msg_compat",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [
            {"type": "tool_use", "id": "toolu_1", "caller": {"type": "direct"}, "name": "lookup", "input": {}}
        ],
        "stop_reason": "tool_use",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 4,
            "server_tool_use": {"web_search_requests": 1, "web_fetch_requests": 0}
        }
    });

    let bytes = serde_json::to_vec(&body).unwrap();
    tracker.scan_bytes(&bytes);
    tracker.finish();

    let projection = &tracker.state.projection;
    assert_eq!(projection.id().as_deref(), Some("msg_compat"));
    assert_eq!(tracker.state.summary.tool_uses.get("lookup"), Some(&1));
}

#[test]
fn tracker_extracts_stream_message_events() {
    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/event-stream"),
    );
    let mut tracker = AnthropicResponseTracker::from_headers(&headers);

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
    tracker.finish();

    let projection = &tracker.state.projection;
    assert_eq!(projection.id().as_deref(), Some("msg_stream"));
    assert_eq!(projection.model().as_deref(), Some("claude-test"));
    assert!(tracker.state.stream_done());
    assert_eq!(projection.output_tokens(), Some(2));

    assert_eq!(tracker.state.summary.stop_reasons.get("end_turn"), Some(&1));
    assert_eq!(
        tracker.state.summary.server_tool_uses.get("web_search"),
        Some(&1)
    );
}

#[test]
fn tracker_normalizes_provider_stream_event_before_observing() {
    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/event-stream"),
    );
    let mut tracker = AnthropicResponseTracker::from_headers(&headers);

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

    let projection = &tracker.state.projection;
    assert_eq!(projection.id().as_deref(), Some("msg_stream"));
    assert_eq!(tracker.state.summary.tool_uses.get("lookup"), Some(&1));
    assert_eq!(
        tracker.state.summary.server_tool_uses.get("web_search"),
        Some(&1)
    );
}
