use serde_json::json;

use super::prepare_anthropic_messages_request;

#[test]
fn preserves_valid_anthropic_messages_request_verbatim() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 256,
        "messages": [{"role": "user", "content": "hello"}],
        "stream": true
    });
    let body = payload.to_string();

    let prepared = prepare_anthropic_messages_request(body.as_bytes()).unwrap();

    assert_eq!(prepared.model, "claude-sonnet-4-5");
    assert_eq!(prepared.normalized_payload, payload);
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
fn does_not_repair_null_anthropic_messages() {
    let body = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 256,
        "messages": null
    })
    .to_string();

    let error = prepare_anthropic_messages_request(body.as_bytes()).unwrap_err();

    assert!(error.to_string().contains("messages"));
}

#[test]
fn accepts_legacy_enabled_thinking_budget_without_rewriting_the_request() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 256,
        "messages": [{"role": "user", "content": "hello"}],
        "thinking": {"type": "enabled", "budget_tokens": 1024}
    });
    let body = payload.to_string();

    let prepared = prepare_anthropic_messages_request(body.as_bytes()).unwrap();

    assert_eq!(prepared.model, "claude-sonnet-4-5");
    assert_eq!(prepared.normalized_payload, payload);
}
