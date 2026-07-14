use std::sync::{Arc, Mutex};

use axum::body::{Body, Bytes, to_bytes};
use futures_util::stream;

use crate::http_support::into_byte_stream;

use super::{
    StreamEvent, StreamTranslationError, StreamTranslationFailureSink, StreamTranslationResult,
    StreamingEventTranslator, translate_sse_stream,
};

struct RejectEveryEvent;

impl StreamingEventTranslator for RejectEveryEvent {
    fn translate_event(
        &mut self,
        _event: StreamEvent,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        Err(StreamTranslationError::Semantic(
            "synthetic failure".to_string(),
        ))
    }
}

#[tokio::test]
async fn failure_sink_receives_the_raw_triggering_sse_event() {
    let failures = Arc::new(Mutex::new(Vec::new()));
    let captured_failures = failures.clone();
    let sink = StreamTranslationFailureSink::new(move |failure| {
        captured_failures.lock().unwrap().push(failure);
    });
    let input = into_byte_stream(stream::iter([Ok::<_, std::io::Error>(Bytes::from_static(
        b"data: {\"type\":\"example\",\"private\":\"redacted\"}\n\n",
    ))]));

    let translated = translate_sse_stream(input, RejectEveryEvent, sink);
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();

    assert!(text.contains("stream translation error"));
    let failures = failures.lock().unwrap();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].stage.as_ref(), "event");
    let event = failures[0].upstream_event.as_ref().unwrap();
    assert_eq!(event.event_type, "message");
    assert_eq!(
        event.data,
        "{\"type\":\"example\",\"private\":\"redacted\"}"
    );
}
