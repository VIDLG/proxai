use serde_json::{Value, json};

use super::{ToolCategory, prepare_provider_request};

fn body(payload: &Value) -> Vec<u8> {
    serde_json::to_vec(payload).unwrap()
}

#[test]
fn prepare_provider_request_preserves_model_when_route_keeps_it() {
    let payload = json!({
        "model": "claude-request",
        "max_tokens": 256,
        "messages": [{"role": "user", "content": "hello"}]
    });

    let prepared = prepare_provider_request(&payload, body(&payload)).unwrap();
    let provider_body = serde_json::from_slice::<serde_json::Value>(&prepared.body).unwrap();

    assert_eq!(provider_body["model"], "claude-request");
}

#[test]
fn prepare_provider_request_uses_provider_payload_model() {
    let payload = json!({
        "model": "claude-upstream",
        "max_tokens": 256,
        "messages": [{"role": "user", "content": "hello"}]
    });

    let prepared = prepare_provider_request(&payload, body(&payload)).unwrap();
    let provider_body = serde_json::from_slice::<serde_json::Value>(&prepared.body).unwrap();

    assert_eq!(provider_body["model"], "claude-upstream");
    assert_eq!(prepared.projection.model, "claude-upstream");
}

#[test]
fn prepare_provider_request_builds_projection_and_summary() {
    let payload = json!({
        "model": "claude-request",
        "max_tokens": 256,
        "stream": true,
        "service_tier": "standard_only",
        "thinking": {"type": "adaptive", "display": "summarized"},
        "tool_choice": {"type": "tool", "name": "lookup"},
        "tools": [
            {
                "type": "custom",
                "name": "lookup",
                "description": "Look up a record",
                "input_schema": {"type": "object", "properties": {}, "required": []}
            },
            {"type": "web_search_20250305"}
        ],
        "messages": [{"role": "user", "content": "hello"}]
    });

    let prepared = prepare_provider_request(&payload, body(&payload)).unwrap();

    assert_eq!(prepared.projection.model, "claude-request");
    assert_eq!(prepared.projection.max_tokens, 256);
    assert_eq!(prepared.projection.stream, Some(true));
    assert_eq!(prepared.summary.tool_inventory.len(), 2);
    assert_eq!(
        prepared.summary.tool_inventory[0].category,
        ToolCategory::Custom
    );
    assert_eq!(prepared.summary.tool_inventory[0].count, 1);
    assert_eq!(prepared.summary.tool_inventory[0].names, ["lookup"]);
    assert_eq!(
        prepared.summary.tool_inventory[1].category,
        ToolCategory::WebSearch
    );
}
