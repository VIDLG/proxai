use super::ResponsesUpstreamTracker;

#[test]
fn records_nested_generic_error_event() {
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/event-stream"),
    );
    let mut tracker = ResponsesUpstreamTracker::from_headers(&headers);

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
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/event-stream"),
    );
    let mut tracker = ResponsesUpstreamTracker::from_headers(&headers);

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
