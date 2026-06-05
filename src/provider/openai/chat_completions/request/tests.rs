use serde_json::{Value, json};

use crate::protocol::openai::chat_completions::{ReasoningEffort, ServiceTier, Verbosity};
use crate::provider::openai::chat_completions::ToolCategory;

use super::prepare_provider_request;

fn body(payload: &Value) -> Vec<u8> {
    serde_json::to_vec(payload).unwrap()
}

#[test]
fn request_projection_uses_async_openai_parse_for_standard_chat_fields() {
    let payload = json!({
        "model": "gpt-4.1",
        "messages": [
            {"role": "system", "content": "You are concise."},
            {"role": "user", "content": "Hello"}
        ],
        "stream": true,
        "max_completion_tokens": 1024,
        "reasoning_effort": "low",
        "verbosity": "high",
        "service_tier": "priority",
        "parallel_tool_calls": true,
        "store": true,
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "lookup",
                    "description": "Lookup data",
                    "parameters": {"type": "object"},
                    "strict": true
                }
            }
        ],
        "tool_choice": {
            "type": "function",
            "function": {"name": "lookup"}
        },
        "response_format": {"type": "json_object"},
        "stream_options": {"include_usage": true, "include_obfuscation": false}
    });

    let prepared = prepare_provider_request(&payload, body(&payload)).unwrap();

    assert_eq!(prepared.projection.model.as_deref(), Some("gpt-4.1"));
    assert_eq!(prepared.projection.stream, Some(true));
    assert_eq!(prepared.projection.max_completion_tokens, Some(1024));
    assert_eq!(
        prepared.projection.reasoning_effort,
        Some(ReasoningEffort::Low)
    );
    assert_eq!(prepared.projection.verbosity, Some(Verbosity::High));
    assert_eq!(
        prepared.projection.service_tier,
        Some(ServiceTier::Priority)
    );
    assert_eq!(prepared.projection.parallel_tool_calls, Some(true));
    assert_eq!(prepared.projection.store, Some(true));
    let stream_options = prepared.projection.stream_options.expect("stream options");
    assert_eq!(stream_options.include_usage, Some(true));
    assert_eq!(stream_options.include_obfuscation, Some(false));
    assert_eq!(prepared.summary.tool_inventory.len(), 1);
    assert_eq!(
        prepared.summary.tool_inventory[0].category,
        ToolCategory::Function
    );
    assert_eq!(prepared.summary.tool_inventory[0].count, 1);
    assert_eq!(
        prepared.summary.tool_inventory[0].names,
        vec!["lookup".to_string()]
    );
}

#[test]
fn prepare_provider_request_uses_provider_payload_model() {
    let payload = json!({
        "model": "gpt-4.1-mini",
        "messages": [{"role": "user", "content": "Hello"}]
    });

    let prepared = prepare_provider_request(&payload, body(&payload)).unwrap();
    let rewritten = serde_json::from_slice::<serde_json::Value>(&prepared.body).unwrap();

    assert_eq!(
        rewritten.get("model").and_then(serde_json::Value::as_str),
        Some("gpt-4.1-mini")
    );
    assert_eq!(prepared.projection.model.as_deref(), Some("gpt-4.1-mini"));
}
