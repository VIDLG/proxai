use serde_json::Value;

use crate::protocol::openai::chat_completions::CreateChatCompletionStreamResponse;

use super::normalize_stream_event_payload;

// Trigger: MiniMax-M3 emitted an OpenAI Chat chunk with Anthropic-style
// service_tier="standard" and omitted required-nullable finish_reason. Symptom:
// Responses -> Chat streaming failed while deserializing service_tier before the
// chunk could be translated. Provenance: diagnostic 1784191897-1784191896360 on
// 2026-07-16; assistant content and the response id were sanitized.
#[test]
fn regression_minimax_standard_service_tier_rejected_in_chat_stream() {
    let event = include_str!(
        "../../../../../../tests/fixtures/regression/minimax-chat-service-tier-standard-event.sse"
    );
    let payload = event
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .expect("fixture must contain one SSE data line");
    let normalized = normalize_stream_event_payload(
        serde_json::from_str::<Value>(payload).expect("fixture data must be JSON"),
    );

    assert_eq!(normalized["service_tier"], "default");
    assert!(normalized["choices"][0]["finish_reason"].is_null());
    serde_json::from_value::<CreateChatCompletionStreamResponse>(normalized)
        .expect("normalized real-world Chat chunk should match the official wire model");
}
