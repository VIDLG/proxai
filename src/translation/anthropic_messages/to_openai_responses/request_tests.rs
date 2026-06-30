use serde_json::json;

use super::super::translate_request_payload;

#[test]
fn translates_basic_anthropic_request_to_responses() {
    let payload = json!({
        "model": "claude-test",
        "max_tokens": 128,
        "messages": [{"role": "user", "content": "hello"}],
        "stream": true,
        "temperature": 0.2,
        "top_p": 0.9,
        "system": "be concise",
        "thinking": {"type": "adaptive", "display": "summarized"},
        "output_config": {
            "effort": "high",
            "format": {"type": "json_schema", "schema": {"type": "object"}}
        },
        "metadata": {"user_id": "user_123"},
        "service_tier": "standard_only",
        "tools": [{
            "type": "custom",
            "name": "lookup",
            "description": "Lookup records",
            "input_schema": {"type": "object"},
            "strict": true
        }],
        "tool_choice": {"type": "tool", "name": "lookup", "disable_parallel_tool_use": true}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["model"], "claude-test");
    assert_eq!(translated["max_output_tokens"], 128);
    assert_eq!(translated["stream"], true);
    assert_eq!(translated["instructions"], "be concise");
    assert_eq!(translated["input"][0]["role"], "user");
    assert_eq!(translated["reasoning"]["effort"], "high");
    assert_eq!(translated["reasoning"]["summary"], "auto");
    assert_eq!(translated["text"]["format"]["type"], "json_schema");
    assert_eq!(translated["metadata"]["user_id"], "user_123");
    assert_eq!(translated["safety_identifier"], "user_123");
    assert_eq!(translated["service_tier"], "default");
    assert_eq!(translated["tools"][0]["type"], "function");
    assert_eq!(translated["tools"][0]["name"], "lookup");
    assert_eq!(translated["tool_choice"]["name"], "lookup");
    assert_eq!(translated["parallel_tool_calls"], false);
}

#[test]
fn translates_output_config_effort_and_thinking_display_to_reasoning() {
    let payload = json!({
        "model": "claude-test",
        "max_tokens": 128,
        "messages": [{"role": "user", "content": "hello"}],
        "output_config": {"effort": "medium"},
        "thinking": {"type": "adaptive", "display": "summarized"}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["reasoning"]["effort"], "medium");
    assert_eq!(translated["reasoning"]["summary"], "auto");
}

#[test]
fn prefers_output_config_effort_over_legacy_thinking_budget() {
    let payload = json!({
        "model": "claude-test",
        "max_tokens": 128,
        "messages": [{"role": "user", "content": "hello"}],
        "output_config": {"effort": "low"},
        "thinking": {"type": "enabled", "budget_tokens": 9000}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["reasoning"]["effort"], "low");
}

#[test]
fn maps_legacy_enabled_thinking_budget_as_reasoning_effort_fallback() {
    let payload = json!({
        "model": "claude-test",
        "max_tokens": 128,
        "messages": [{"role": "user", "content": "hello"}],
        "thinking": {"type": "enabled", "budget_tokens": 9000}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["reasoning"]["effort"], "high");
}

#[test]
fn rejects_anthropic_container_upload_for_responses_input() {
    let payload = json!({
        "model": "claude-test",
        "max_tokens": 128,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "container_upload",
                "file_id": "file_anthropic_container"
            }]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("container_upload content cannot be translated"));
}
