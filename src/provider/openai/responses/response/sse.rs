use crate::sse::SseEvent;

const TERMINAL_EVENT_TYPES: &[&str] = &[
    "response.function_call_arguments.done",
    "response.completed",
    "response.incomplete",
    "response.failed",
    "response.error",
];

pub(super) fn is_tool_argument_delta(event: &SseEvent) -> bool {
    event.matches_type_or_data("response.function_call_arguments.delta")
}

pub(super) fn is_tool_argument_done(event: &SseEvent) -> bool {
    event.matches_type_or_data("response.function_call_arguments.done")
}

pub(super) fn is_terminal_event(event: &SseEvent) -> bool {
    TERMINAL_EVENT_TYPES
        .iter()
        .copied()
        .any(|event_type| event.matches_type_or_data(event_type))
        || event.data.contains("\"type\":\"error\"")
        || event.data.contains("\"type\": \"error\"")
}

#[cfg(test)]
#[path = "sse_tests.rs"]
mod tests;
