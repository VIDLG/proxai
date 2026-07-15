use serde_json::json;

use crate::protocol::ProviderProtocol;

use super::normalize_provider_error;

#[test]
fn normalizes_openai_error_shape() {
    let error = normalize_provider_error(
        ProviderProtocol::OpenaiResponses,
        &json!({
            "error": {
                "code": "array_above_max_length",
                "message": "input array is too long",
                "param": "input[3].content",
                "type": "invalid_request_error"
            }
        }),
    )
    .unwrap();

    assert_eq!(error.code.as_deref(), Some("array_above_max_length"));
    assert_eq!(error.message, "input array is too long");
    assert_eq!(error.param, Some(json!("input[3].content")));
}

#[test]
fn normalizes_anthropic_error_type_as_code() {
    let error = normalize_provider_error(
        ProviderProtocol::AnthropicMessages,
        &json!({
            "type": "error",
            "error": {
                "type": "rate_limit_error",
                "message": "rate limit exceeded"
            }
        }),
    )
    .unwrap();

    assert_eq!(error.code.as_deref(), Some("rate_limit_error"));
    assert_eq!(error.message, "rate limit exceeded");
    assert_eq!(error.param, None);
}

#[test]
fn accepts_root_compatibility_shape() {
    let error = normalize_provider_error(
        ProviderProtocol::OpenaiChatCompletions,
        &json!({
            "code": "model_not_found",
            "message": "unknown model",
            "param": {"model": "missing"}
        }),
    )
    .unwrap();

    assert_eq!(error.code.as_deref(), Some("model_not_found"));
    assert_eq!(error.message, "unknown model");
    assert_eq!(error.param, Some(json!({"model": "missing"})));
}

#[test]
fn accepts_string_message_fallbacks() {
    for payload in [
        json!({"error": "provider unavailable"}),
        json!({"detail": "provider unavailable"}),
        json!({"message": "provider unavailable"}),
    ] {
        let error = normalize_provider_error(ProviderProtocol::OpenaiResponses, &payload).unwrap();
        assert_eq!(error.message, "provider unavailable");
    }
}

#[test]
fn joins_fastapi_detail_messages() {
    let error = normalize_provider_error(
        ProviderProtocol::OpenaiResponses,
        &json!({
            "detail": [
                {"msg": "model is required"},
                {"message": "input is invalid"},
                {"loc": ["body", "input"]}
            ]
        }),
    )
    .unwrap();

    assert_eq!(error.message, "model is required; input is invalid");
}

#[test]
fn rejects_unknown_structured_shape() {
    assert_eq!(
        normalize_provider_error(
            ProviderProtocol::OpenaiResponses,
            &json!({"unexpected": true}),
        ),
        None
    );
}
