use super::{analyze_unfinished_tool_tail, ToolArgumentsParseResult};
use serde_json::json;

#[test]
fn unfinished_tool_diagnostic_detects_complete_json_without_done_event() {
    let tail = b"event: response.function_call_arguments.delta\n\
 data: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"{\\\"path\\\":\\\"\",\"item_id\":\"fc_test\",\"sequence_number\":10}\n\n\
 event: response.function_call_arguments.delta\n\
 data: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"proxai/src/logging/mod.rs\\\"}\",\"item_id\":\"fc_test\",\"sequence_number\":11}\n\n";

    let diagnostic = analyze_unfinished_tool_tail(tail);

    assert_eq!(diagnostic.item_id.as_deref(), Some("fc_test"));
    assert_eq!(
        diagnostic.assembled,
        "{\"path\":\"proxai/src/logging/mod.rs\"}"
    );
    assert_eq!(
        diagnostic.parsed,
        ToolArgumentsParseResult::Json {
            value: json!({"path": "proxai/src/logging/mod.rs"})
        }
    );
    assert!(!diagnostic.saw_arguments_done);
    assert!(!diagnostic.saw_terminal_event);
    assert_eq!(diagnostic.last_sequence_number, Some(11));
}
