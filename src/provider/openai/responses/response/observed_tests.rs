use super::super::summary::ResponseSummary;
use super::{ObservedState, ObservedUpdate};
use crate::provider::openai::responses::ResponseOutputItemKind;

#[test]
fn observed_updates_merge_later_function_call_name() {
    let mut state = ObservedState::default();

    state.apply(&ObservedUpdate::FunctionCall {
        id: "fc_test".to_string(),
        name: "lookup".to_string(),
    });
    state.apply(&ObservedUpdate::FunctionCall {
        id: "fc_test".to_string(),
        name: "ignored_later_duplicate".to_string(),
    });

    let summary = ResponseSummary::from(&state);

    assert_eq!(
        summary
            .output_items
            .get(&ResponseOutputItemKind::FunctionCall),
        Some(&1)
    );
    assert_eq!(summary.function_calls.get("lookup"), Some(&1));
    assert_eq!(summary.function_calls.get("ignored_later_duplicate"), None);
}

#[test]
fn observed_updates_merge_later_mcp_call_details() {
    let mut state = ObservedState::default();

    state.apply(&ObservedUpdate::from_mcp_call_lifecycle("mcp_test"));
    state.apply(&ObservedUpdate::McpCall {
        id: "mcp_test".to_string(),
        server_label: Some("github".to_string()),
        name: Some("search".to_string()),
    });

    let summary = ResponseSummary::from(&state);

    assert_eq!(
        summary.output_items.get(&ResponseOutputItemKind::McpCall),
        Some(&1)
    );
    assert_eq!(summary.mcp_calls.get("github/search"), Some(&1));
}

#[test]
fn summary_only_updates_are_deduped_by_output_index() {
    let mut state = ObservedState::default();
    let update = ObservedUpdate::SummaryOnlyItemKind {
        kind: ResponseOutputItemKind::ToolSearchCall,
        event_key: "tool_search_call:7".to_string(),
    };

    state.apply(&update);
    state.apply(&update);

    let summary = ResponseSummary::from(&state);

    assert_eq!(
        summary
            .output_items
            .get(&ResponseOutputItemKind::ToolSearchCall),
        Some(&1)
    );
}
