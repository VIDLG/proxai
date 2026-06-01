use crate::sse::SseEvent;

use super::{is_terminal_event, is_tool_argument_delta, is_tool_argument_done};

#[test]
fn classifies_terminal_and_tool_stream_events() {
    let explicit_terminal = SseEvent {
        event_type: "response.completed".to_string(),
        data: "{}".to_string(),
    };
    let data_only_terminal = SseEvent {
        event_type: SseEvent::DEFAULT_EVENT_TYPE.to_string(),
        data: "{\"type\":\"response.error\"}".to_string(),
    };
    let tool_delta = SseEvent {
        event_type: "response.function_call_arguments.delta".to_string(),
        data: "{}".to_string(),
    };
    let tool_done = SseEvent {
        event_type: SseEvent::DEFAULT_EVENT_TYPE.to_string(),
        data: "{\"type\":\"response.function_call_arguments.done\"}".to_string(),
    };

    assert!(is_terminal_event(&explicit_terminal));
    assert!(is_terminal_event(&data_only_terminal));
    assert!(is_tool_argument_delta(&tool_delta));
    assert!(is_tool_argument_done(&tool_done));
}
