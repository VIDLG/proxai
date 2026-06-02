use super::OpenaiResponsesUpstreamBodyObserver;
use crate::provider::BodyObserver;

fn test_observer(headers: &http::HeaderMap) -> OpenaiResponsesUpstreamBodyObserver {
    OpenaiResponsesUpstreamBodyObserver::new(headers, None, 1, tracing::Span::none())
}

#[test]
fn sse_eof_without_terminal_event_is_incomplete_even_without_pending_tools() {
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/event-stream"),
    );
    let mut observer = test_observer(&headers);

    observer.observe_chunk(
        br#"data: {"type":"response.output_text.delta","sequence_number":1,"delta":"ok"}

"#,
    );

    assert!(!observer.saw_terminal_event);
    assert!(observer.stream_error.is_none());
}

#[test]
fn sse_eof_after_terminal_event_is_complete() {
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/event-stream"),
    );
    let mut observer = test_observer(&headers);

    observer.observe_chunk(
        br#"data: {"type":"response.completed","sequence_number":2}

"#,
    );

    assert!(observer.saw_terminal_event);
    assert!(observer.stream_error.is_none());
}

#[test]
fn tool_argument_delta_without_item_id_marks_stream_error() {
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/event-stream"),
    );
    let mut observer = test_observer(&headers);

    observer.observe_chunk(
        br#"data: {"type":"response.function_call_arguments.delta","sequence_number":1,"delta":"{}"}

"#,
    );

    assert!(observer.saw_terminal_event);
    assert!(observer.stream_error.is_some());
}
