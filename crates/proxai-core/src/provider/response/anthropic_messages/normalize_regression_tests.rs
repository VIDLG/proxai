use serde_json::Value;

use crate::protocol::anthropic::messages::{MessageStreamEvent, StopReason};

use super::normalize_stream_event_payload;

// Trigger: an Anthropic-compatible upstream emitted the Beta-defined
// `model_context_window_exceeded` stop reason on a stable `message_delta`.
// Symptom: stream translation failed while deserializing `StopReason` before a
// terminal event could be projected. Provenance: diagnostic
// 1784598362-1784598351961 on 2026-07-21; the captured event contained no prompt
// content or identifiers and is committed verbatim.
#[test]
fn regression_anthropic_model_context_window_exceeded_stop_reason_rejected() {
    let event = include_str!(
        "../../../../../../tests/fixtures/regression/anthropic-model-context-window-exceeded-event.sse"
    );
    let data = event
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .expect("fixture should contain an SSE data line");
    let payload: Value = serde_json::from_str(data).expect("fixture data should be valid JSON");

    let parsed: MessageStreamEvent =
        serde_json::from_value(normalize_stream_event_payload(payload))
            .expect("normalized provider event should match the Anthropic wire model");

    assert!(matches!(
        parsed,
        MessageStreamEvent::MessageDelta(event)
            if matches!(
                event.delta.stop_reason.as_non_null(),
                Some(StopReason::ModelContextWindowExceeded)
            )
    ));
}
