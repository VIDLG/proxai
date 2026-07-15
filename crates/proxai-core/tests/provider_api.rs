use proxai_core::protocol::ProviderProtocol;
use proxai_core::provider::{ProviderCompatibility, ProviderNormalizer, ProviderRequestPreparer};
use proxai_core::translation::stream::StreamEvent;
use serde_json::json;

#[test]
fn prepares_provider_request_values_through_the_public_api() {
    let prepared = ProviderRequestPreparer::new(ProviderProtocol::OpenaiResponses).prepare(
        json!({
            "model": "client-model",
            "input": [{"type": "reasoning", "status": "completed", "content": []}]
        }),
        "upstream-model",
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

    let normalized = ProviderNormalizer::new(
        ProviderProtocol::OpenaiChatCompletions,
        ProviderCompatibility::Compatible,
    )
    .normalize_stream_event(event);

    assert_eq!(normalized.data["choices"][0]["finish_reason"], json!(null));
}
