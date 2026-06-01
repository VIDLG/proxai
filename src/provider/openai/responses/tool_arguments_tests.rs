use crate::sse::SseEvent;
use serde_json::json;

use super::{tool_argument_item_id, ToolArgumentStreamState};

#[test]
fn tool_argument_item_id_reads_non_empty_item_id() {
    assert_eq!(
        tool_argument_item_id(&json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_test",
            "delta": "{}",
        })),
        Some("fc_test".to_string())
    );
}

#[test]
fn tool_argument_item_id_ignores_missing_or_empty_item_id() {
    assert_eq!(tool_argument_item_id(&json!({})), None);
    assert_eq!(tool_argument_item_id(&json!({ "item_id": "" })), None);
}

#[test]
fn tracks_tool_argument_delta_and_done_by_item_id() {
    let mut state = ToolArgumentStreamState::default();

    state
        .observe_event(
            &SseEvent {
                event_type: "response.function_call_arguments.delta".to_string(),
                data: r#"{"item_id":"fc_1","delta":"{}"}"#.to_string(),
            },
            None,
        )
        .unwrap();
    assert!(state.has_pending_items());

    state
        .observe_event(
            &SseEvent {
                event_type: "response.function_call_arguments.done".to_string(),
                data: r#"{"item_id":"fc_1"}"#.to_string(),
            },
            None,
        )
        .unwrap();
    assert!(!state.has_pending_items());
}

#[test]
fn delta_without_item_id_is_malformed() {
    let mut state = ToolArgumentStreamState::default();

    let error = state
        .observe_event(
            &SseEvent {
                event_type: "response.function_call_arguments.delta".to_string(),
                data: r#"{"delta":"{}"}"#.to_string(),
            },
            None,
        )
        .expect_err("missing item_id should be malformed");

    assert_eq!(
        error,
        "upstream Responses SSE tool argument delta missing non-empty item_id"
    );
    assert!(!state.has_pending_items());
}

#[test]
fn terminal_clear_resets_pending() {
    let mut state = ToolArgumentStreamState::default();

    state
        .observe_event(
            &SseEvent {
                event_type: "response.function_call_arguments.delta".to_string(),
                data: r#"{"item_id":"fc_1","delta":"{}"}"#.to_string(),
            },
            None,
        )
        .unwrap();
    assert!(state.has_pending_items());

    state.clear();

    assert!(!state.has_pending_items());
    assert!(state.timeout_sleep_mut().is_none());
}
