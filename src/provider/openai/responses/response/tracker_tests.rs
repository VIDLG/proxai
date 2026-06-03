use super::ResponsesUpstreamTracker;

#[test]
fn records_nested_generic_error_event() {
    let mut tracker = ResponsesUpstreamTracker::new();

    tracker.scan_bytes(
        br#"event: error
data: {"type":"error","error":{"type":"invalid_request_error","code":"context_length_exceeded","message":"Your input exceeds the context window of this model.","param":"input"},"sequence_number":2}

"#,
    );

    let error = tracker.state.observed_error().unwrap();
    assert_eq!(error.code, "context_length_exceeded");
    assert_eq!(
        error.message,
        "Your input exceeds the context window of this model."
    );
    assert_eq!(tracker.state.sequence_number, Some(2));
}

#[test]
fn nested_generic_error_overrides_in_progress_snapshot_for_diagnostics() {
    let mut tracker = ResponsesUpstreamTracker::new();

    tracker.scan_bytes(
        br#"event: response.created
data: {"type":"response.created","response":{"id":"resp_1","object":"response","created_at":0,"status":"in_progress","model":"gpt-5.5","output":[],"parallel_tool_calls":true,"tool_choice":"auto","tools":[]},"sequence_number":1}

"#,
    );
    tracker.scan_bytes(
        br#"event: error
data: {"type":"error","error":{"type":"invalid_request_error","code":"context_length_exceeded","message":"Your input exceeds the context window of this model.","param":"input"},"sequence_number":2}

"#,
    );

    let error = tracker.state.effective_error().unwrap();
    assert_eq!(error.code, "context_length_exceeded");
    assert_eq!(
        error.message,
        "Your input exceeds the context window of this model."
    );
    assert_eq!(tracker.state.sequence_number, Some(2));
}

#[test]
fn completed_snapshot_without_output_uses_fallback_summary() {
    let mut tracker = ResponsesUpstreamTracker::new();

    tracker.scan_bytes(
        br#"event: response.output_item.done
data: {"type":"response.output_item.done","sequence_number":3,"output_index":0,"item":{"id":"fc_1","type":"function_call","name":"edit_file","call_id":"call_1","arguments":"{}"}}

"#,
    );
    tracker.scan_bytes(
        br#"event: response.completed
data: {"type":"response.completed","sequence_number":4,"response":{"id":"resp_1","object":"response","created_at":0,"model":"gpt-5.5","status":"completed","output":[]}}

"#,
    );

    let summary = tracker.state.effective_summary();

    assert_eq!(summary.function_calls.get("edit_file"), Some(&1));
}
