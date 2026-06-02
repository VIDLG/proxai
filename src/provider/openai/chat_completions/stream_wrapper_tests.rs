use axum::http::HeaderMap;

use super::ChatUpstreamBodyObserver;
use crate::provider::BodyObserver;

fn sse_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/event-stream"),
    );
    headers
}

#[test]
fn sse_eof_without_done_sentinel_is_incomplete() {
    let mut observer = ChatUpstreamBodyObserver::new(
        super::ChatUpstreamResponseTracker::from_headers(&sse_headers()),
        tracing::Span::none(),
    );

    observer.observe_chunk(
        br#"data: {"id":"chatcmpl_stream","object":"chat.completion.chunk","created":1,"model":"gpt-4.1","choices":[{"index":0,"delta":{"content":"hi"},"finish_reason":null}]}

"#,
    );

    assert!(!observer.tracker.state.stream_done);
    assert!(observer.error_message.is_none());
}

#[test]
fn sse_eof_after_done_sentinel_is_complete() {
    let mut observer = ChatUpstreamBodyObserver::new(
        super::ChatUpstreamResponseTracker::from_headers(&sse_headers()),
        tracing::Span::none(),
    );

    observer.observe_chunk(b"data: [DONE]\n\n");

    assert!(observer.tracker.state.stream_done);
    assert!(observer.error_message.is_none());
}
