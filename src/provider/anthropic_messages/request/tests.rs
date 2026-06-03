use serde_json::json;

use super::{prepare_provider_request, ToolCategory};

#[test]
fn prepare_provider_request_preserves_model_when_route_keeps_it() {
    let payload = json!({
        "model": "claude-request",
        "max_tokens": 256,
        "messages": [{"role": "user", "content": "hello"}]
    });

    let prepared = prepare_provider_request(&payload, "claude-request", "claude-request").unwrap();
    let provider_body = serde_json::from_slice::<serde_json::Value>(&prepared.body).unwrap();

    assert_eq!(provider_body["model"], "claude-request");
}

#[test]
fn prepare_provider_request_rewrites_model() {
    let payload = json!({
        "model": "claude-request",
        "max_tokens": 256,
        "messages": [{"role": "user", "content": "hello"}]
    });

    let prepared = prepare_provider_request(&payload, "claude-request", "claude-upstream").unwrap();
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
        "thinking": {"type": "enabled", "budget_tokens": 1024},
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

    let prepared = prepare_provider_request(&payload, "claude-request", "claude-request").unwrap();

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
