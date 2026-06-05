use super::ChatUpstreamBodyObserver;
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

#[test]
fn sse_eof_without_done_sentinel_is_incomplete() {
    let mut observer = ChatUpstreamBodyObserver::new(test_obs());

    observer.on_chunk(
        br#"data: {"id":"chatcmpl_stream","object":"chat.completion.chunk","created":1,"model":"gpt-4.1","choices":[{"index":0,"delta":{"content":"hi"},"finish_reason":null}]}

"#,
    );

    assert!(!observer.state.stream_done);
    assert!(observer.stream_error.is_none());
}

#[test]
fn sse_eof_after_done_sentinel_is_complete() {
    let mut observer = ChatUpstreamBodyObserver::new(test_obs());

    observer.on_chunk(b"data: [DONE]\n\n");

    assert!(observer.state.stream_done);
    assert!(observer.stream_error.is_none());
}
