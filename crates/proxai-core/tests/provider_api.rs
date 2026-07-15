use proxai_core::observe::NoopObserver;
use proxai_core::protocol::ProviderProtocol;
use proxai_core::provider::{
    ProviderBehavior, ProviderCompatibility, normalize_provider_error,
    normalize_provider_stream_event, prepare_provider_request,
};
use proxai_core::translation::stream::StreamEvent;
use serde_json::json;

#[test]
fn normalizes_provider_errors_through_the_public_api() {
    let error = normalize_provider_error(
        ProviderProtocol::AnthropicMessages,
        &json!({
            "type": "error",
            "error": {"type": "authentication_error", "message": "invalid API key"}
        }),
    )
    .unwrap();

    assert_eq!(error.code.as_deref(), Some("authentication_error"));
    assert_eq!(error.message, "invalid API key");
}

#[test]
fn prepares_provider_request_values_through_the_public_api() {
    let prepared = prepare_provider_request(
        ProviderProtocol::OpenaiResponses,
        json!({
            "model": "client-model",
            "input": [{"type": "reasoning", "status": "completed", "content": []}]
        }),
        "upstream-model",
        &NoopObserver,
    );

    assert_eq!(prepared["model"], "upstream-model");
    assert!(prepared["input"][0].get("status").is_none());
    assert!(prepared["input"][0].get("content").is_none());
}

#[test]
fn normalizes_provider_stream_events_through_the_public_api() {
    let event = StreamEvent::message(json!({
        "choices": [{"index": 0, "delta": {"content": "hello"}}]
    }))
    .unwrap();

    let normalized = normalize_provider_stream_event(
        ProviderBehavior::new(
            ProviderProtocol::OpenaiChatCompletions,
            ProviderCompatibility::Compatible,
        ),
        event,
        &NoopObserver,
    );

    assert_eq!(normalized.data["choices"][0]["finish_reason"], json!(null));
}
