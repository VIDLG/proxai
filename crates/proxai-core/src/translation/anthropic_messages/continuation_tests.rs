use super::{Continuation, ContinuationEnvelope};

fn thinking() -> Continuation {
    Continuation::Thinking {
        thinking: "plan".to_string(),
        signature: "sig".to_string(),
    }
}

#[test]
fn round_trips_responses_continuation_envelope() {
    let encoded = ContinuationEnvelope::from(vec![thinking()])
        .encode()
        .unwrap();

    assert_eq!(
        ContinuationEnvelope::decode(&encoded).unwrap(),
        Some(ContinuationEnvelope::from(vec![thinking()]))
    );
}

#[test]
fn separates_chat_thinking_from_continuation_envelope() {
    let encoded = ContinuationEnvelope::from(vec![thinking()])
        .append_to_chat_reasoning_content("visible thinking".to_string())
        .unwrap();

    assert_eq!(
        ContinuationEnvelope::split_chat_reasoning_content(&encoded).unwrap(),
        (
            "visible thinking".to_string(),
            Some(ContinuationEnvelope::from(vec![thinking()])),
        ),
    );
}

#[test]
fn leaves_chat_thinking_without_an_envelope_unchanged() {
    assert_eq!(
        ContinuationEnvelope::split_chat_reasoning_content("ordinary thinking").unwrap(),
        ("ordinary thinking".to_string(), None),
    );
}
