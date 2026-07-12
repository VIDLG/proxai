use serde_json::json;

use super::prepare_openai_chat_completions_request;

#[test]
fn preserves_valid_chat_completions_request_verbatim() {
    let payload = json!({
        "model": "gpt-4.1",
        "messages": [{"role": "user", "content": "hello"}],
        "stream": true
    });
    let body = payload.to_string();

    let prepared = prepare_openai_chat_completions_request(body.as_bytes()).unwrap();

    assert_eq!(prepared.model, "gpt-4.1");
    assert_eq!(prepared.normalized_payload, payload);
}

#[test]
fn rejects_chat_completions_request_without_model() {
    let body = json!({
        "messages": [{"role": "user", "content": "hello"}]
    })
    .to_string();

    let error = prepare_openai_chat_completions_request(body.as_bytes()).unwrap_err();

    assert!(error.to_string().contains("model"));
}

#[test]
fn does_not_repair_null_chat_completions_messages() {
    let body = json!({
        "model": "gpt-4.1",
        "messages": null
    })
    .to_string();

    let error = prepare_openai_chat_completions_request(body.as_bytes()).unwrap_err();

    assert!(error.to_string().contains("messages"));
}
