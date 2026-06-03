use super::super::tracker::AnthropicResponseTracker;
use super::AnthropicSseObserver;
use crate::upstream::BodyObserver;

fn test_obs() -> crate::observe::ObserveContext {
    let request_id = crate::request::RequestId::from(1);
    crate::observe::ObserveContext::new(
        request_id,
        std::time::Instant::now(),
        crate::observe::CaptureController::new(None, crate::config::CaptureConfig::default())
            .session(request_id),
        tracing::Span::none(),
    )
}

fn sse_tracker() -> AnthropicResponseTracker {
    AnthropicResponseTracker::new()
}

#[test]
fn sse_eof_without_message_stop_is_incomplete() {
    let mut observer = AnthropicSseObserver::new(sse_tracker(), test_obs());

    observer.observe_chunk(
        br#"data: {"type":"message_start","message":{"id":"msg_stream","type":"message","role":"assistant","model":"claude-test","content":[],"stop_reason":null,"stop_sequence":null,"stop_details":null,"container":null,"usage":{"input_tokens":8,"output_tokens":0,"cache_creation":null,"cache_creation_input_tokens":null,"cache_read_input_tokens":null,"inference_geo":null,"server_tool_use":null,"service_tier":"standard"}}}

"#,
    );

    assert!(!observer.saw_terminal);
    assert!(observer.stream_error.is_none());
}

#[test]
fn sse_eof_after_message_stop_is_complete() {
    let mut observer = AnthropicSseObserver::new(sse_tracker(), test_obs());

    observer.observe_chunk(
        br#"data: {"type":"message_stop"}

"#,
    );

    assert!(observer.saw_terminal);
    assert!(observer.stream_error.is_none());
}
