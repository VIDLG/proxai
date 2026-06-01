use axum::http::HeaderMap;

use super::{AnthropicResponseTracker, AnthropicSseObserver};
use crate::provider::BodyObserver;

fn sse_tracker() -> AnthropicResponseTracker {
    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/event-stream"),
    );
    AnthropicResponseTracker::from_headers(&headers)
}

#[test]
fn sse_eof_without_message_stop_is_incomplete() {
    let mut observer = AnthropicSseObserver::new(sse_tracker(), tracing::Span::none());

    observer.observe_chunk(
        br#"data: {"type":"message_start","message":{"id":"msg_stream","type":"message","role":"assistant","model":"claude-test","content":[],"stop_reason":null,"stop_sequence":null,"stop_details":null,"container":null,"usage":{"input_tokens":8,"output_tokens":0,"cache_creation":null,"cache_creation_input_tokens":null,"cache_read_input_tokens":null,"inference_geo":null,"server_tool_use":null,"service_tier":"standard"}}}

"#,
    );

    assert!(!observer.is_terminal());
    assert!(!observer.is_error());
}

#[test]
fn sse_eof_after_message_stop_is_complete() {
    let mut observer = AnthropicSseObserver::new(sse_tracker(), tracing::Span::none());

    observer.observe_chunk(
        br#"data: {"type":"message_stop"}

"#,
    );

    assert!(observer.is_terminal());
    assert!(!observer.is_error());
}
