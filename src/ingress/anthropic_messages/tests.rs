use serde_json::json;

use super::prepare_anthropic_messages_request;

#[test]
fn prepares_anthropic_messages_request_with_schema_parse() {
    let body = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 256,
        "messages": [{"role": "user", "content": "hello"}],
        "stream": true
    })
    .to_string();

    let prepared = prepare_anthropic_messages_request(body.as_bytes()).unwrap();

    assert_eq!(prepared.model, "claude-sonnet-4-5");
    assert_eq!(prepared.normalized_payload["stream"], true);
}

#[test]
fn rejects_anthropic_messages_request_without_messages() {
    let body = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 256
    })
    .to_string();

    let error = prepare_anthropic_messages_request(body.as_bytes()).unwrap_err();

    assert!(error.to_string().contains("messages"));
}

#[test]
fn rejects_anthropic_messages_request_without_model() {
    let body = json!({
        "max_tokens": 256,
        "messages": [{"role": "user", "content": "hello"}]
    })
    .to_string();

    let error = prepare_anthropic_messages_request(body.as_bytes()).unwrap_err();

    assert!(error.to_string().contains("model"));
}

#[test]
fn accepts_legacy_enabled_thinking_budget() {
    let body = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 256,
        "messages": [{"role": "user", "content": "hello"}],
        "thinking": {"type": "enabled", "budget_tokens": 1024}
    })
    .to_string();

    let prepared = prepare_anthropic_messages_request(body.as_bytes()).unwrap();

    assert_eq!(prepared.model, "claude-sonnet-4-5");
}
