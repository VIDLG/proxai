use super::{ObservedChatState, ObservedChatUpdate};

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
