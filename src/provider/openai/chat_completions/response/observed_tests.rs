use super::{ChatResponseObservation, ObservedChatState, ObservedChatUpdate};
use crate::protocol::openai::chat_completions::{
    ChatResponseProjection, CreateChatCompletionResponse, FinishReason,
};
use serde_json::json;

#[test]
fn observed_chat_state_deduplicates_stream_tool_call_deltas() {
    let mut state = ObservedChatState::default();

    state.apply(&ObservedChatUpdate::Choice { index: 0 });
    state.apply(&ObservedChatUpdate::Text { index: 0 });
    state.apply(&ObservedChatUpdate::ToolCall {
        choice_index: 0,
        tool_index: 0,
        name: Some("lookup".to_string()),
    });
    state.apply(&ObservedChatUpdate::ToolCall {
        choice_index: 0,
        tool_index: 0,
        name: None,
    });
    state.apply(&ObservedChatUpdate::FinishReason {
        index: 0,
        reason: "stop".to_string(),
    });
    state.apply(&ObservedChatUpdate::FinishReason {
        index: 0,
        reason: "stop".to_string(),
    });

    let summary = state.fallback_summary();

    assert_eq!(summary.output_items.values().sum::<u64>(), 4);
    assert_eq!(summary.tool_call_names.get("lookup"), Some(&1));
    assert_eq!(summary.finish_reasons.get("stop"), Some(&1));
}

#[test]
fn non_stream_chat_completion_observation_summarizes_usage() {
    let body = json!({
        "id": "chatcmpl_123",
        "object": "chat.completion",
        "created": 1,
        "model": "gpt-4.1",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "ok",
                "tool_calls": [{
                    "type": "function",
                    "id": "call_123",
                    "function": {"name": "lookup", "arguments": "{}"}
                }]
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 3,
            "total_tokens": 13,
            "prompt_tokens_details": {"cached_tokens": 4},
            "completion_tokens_details": {"reasoning_tokens": 2}
        }
    });

    let response = serde_json::from_value::<CreateChatCompletionResponse>(body).unwrap();
    let observation = ChatResponseObservation::NonStream(ChatResponseProjection::from(response));
    let ChatResponseObservation::NonStream(projection) = &observation else {
        panic!("expected non-stream chat completion projection");
    };

    assert_eq!(projection.id, "chatcmpl_123");
    assert_eq!(projection.model, "gpt-4.1");
    assert_eq!(projection.choices.len(), 1);
    assert_eq!(
        projection.choices[0].finish_reason,
        Some(FinishReason::Stop)
    );
    let usage = projection.usage.as_ref().expect("usage");
    assert_eq!(usage.total_tokens, 13);
    assert_eq!(
        usage
            .prompt_tokens_details
            .and_then(|details| details.cached_tokens),
        Some(4)
    );
    assert_eq!(
        usage
            .completion_tokens_details
            .and_then(|details| details.reasoning_tokens),
        Some(2)
    );

    let summary = observation.summary();
    assert_eq!(summary.output_items.values().sum::<u64>(), 4);
    assert_eq!(summary.finish_reasons.get("stop"), Some(&1));
    assert_eq!(summary.tool_call_names.get("lookup"), Some(&1));
}
