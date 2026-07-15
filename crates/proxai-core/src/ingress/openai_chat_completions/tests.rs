use serde_json::json;

use crate::ingress::IngressError;

use super::prepare_openai_chat_completions_request;

#[test]
fn preserves_valid_chat_completions_request_verbatim() {
    let payload = json!({
        "model": "gpt-4.1",
        "messages": [{"role": "user", "content": "hello"}],
        "stream": true
    });

    let prepared = prepare_openai_chat_completions_request(payload.clone()).unwrap();

    assert_eq!(prepared.model(), "gpt-4.1");
    assert_eq!(prepared.normalized_payload(), &payload);
}

#[test]
fn rejects_chat_completions_request_without_model() {
    let payload = json!({
        "messages": [{"role": "user", "content": "hello"}]
    });

    let error = prepare_openai_chat_completions_request(payload).unwrap_err();

    let IngressError::JsonPayload(error) = error else {
        panic!("expected a typed JSON payload error");
    };
    assert_eq!(error.path(), ".");
    assert!(error.to_string().contains("model"));
}

#[test]
fn does_not_repair_null_chat_completions_messages() {
    let payload = json!({
        "model": "gpt-4.1",
        "messages": null
    });

    let error = prepare_openai_chat_completions_request(payload).unwrap_err();

    let IngressError::JsonPayload(error) = error else {
        panic!("expected a typed JSON payload error");
    };
    assert_eq!(error.path(), "messages");
}
