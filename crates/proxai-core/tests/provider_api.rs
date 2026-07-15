use proxai_core::protocol::ProviderProtocol;
use proxai_core::provider::{ProviderCompatibility, ProviderNormalizer};
use proxai_core::translation::stream::StreamEvent;
use serde_json::json;

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
