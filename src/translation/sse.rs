use axum::body::{Body, Bytes};
use axum::http::{header, Response};
use futures_util::{stream, Stream, StreamExt};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::io;
use std::pin::Pin;

pub(crate) use crate::sse::encode_sse_json;
use crate::sse::{SseEvent, SseEventScanner};

pub(crate) trait SseEventTranslator: Send + 'static {
    fn translate_event(&mut self, event: SseEvent) -> io::Result<Vec<Bytes>>;

    fn finish(&mut self) -> io::Result<Vec<Bytes>> {
        Ok(Vec::new())
    }
}

pub(crate) fn translate_sse_response<T>(response: Response<Body>, translator: T) -> Response<Body>
where
    T: SseEventTranslator,
{
    translate_sse_response_with_error_encoder(response, translator, encode_error_event)
}

pub(crate) fn translate_sse_response_with_error_encoder<T>(
    response: Response<Body>,
    translator: T,
    error_encoder: fn(&str, io::Error) -> io::Result<Bytes>,
) -> Response<Body>
where
    T: SseEventTranslator,
{
    let (mut parts, body) = response.into_parts();
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );

    let stream = translate_sse_body(body, translator, error_encoder);
    Response::from_parts(parts, Body::from_stream(stream))
}

pub(crate) fn event_payload_with_type(event: &SseEvent) -> io::Result<Value> {
    event.payload_with_type()
}

fn translate_sse_body<T>(
    body: Body,
    translator: T,
    error_encoder: fn(&str, io::Error) -> io::Result<Bytes>,
) -> impl Stream<Item = io::Result<Bytes>> + Send + 'static
where
    T: SseEventTranslator,
{
    let state = SseTranslationState {
        input: Box::pin(body.into_data_stream()),
        scanner: SseEventScanner::default(),
        translator,
        error_encoder,
        pending: VecDeque::new(),
        finished_input: false,
        closed: false,
    };

    stream::unfold(state, |mut state| async move {
        loop {
            if let Some(chunk) = state.pending.pop_front() {
                return Some((Ok(chunk), state));
            }
            if state.closed {
                return None;
            }
            if state.finished_input {
                match state.translator.finish() {
                    Ok(chunks) => {
                        state.pending.extend(chunks);
                        state.closed = true;
                        continue;
                    }
                    Err(error) => {
                        state.closed = true;
                        return Some((
                            (state.error_encoder)("SSE translation finish error", error),
                            state,
                        ));
                    }
                }
            }

            match state.input.next().await {
                Some(Ok(chunk)) => {
                    for event in state.scanner.scan(&chunk) {
                        match state.translator.translate_event(event) {
                            Ok(chunks) => state.pending.extend(chunks),
                            Err(error) => {
                                state.closed = true;
                                return Some((
                                    (state.error_encoder)("SSE translation error", error),
                                    state,
                                ));
                            }
                        }
                    }
                }
                Some(Err(error)) => {
                    state.closed = true;
                    let payload = json!({
                        "type": "error",
                        "error": {
                            "message": format!("upstream SSE stream error: {error}")
                        }
                    });
                    return Some((encode_sse_json("error", &payload), state));
                }
                None => {
                    state.finished_input = true;
                }
            }
        }
    })
}

fn encode_error_event(context: &str, error: io::Error) -> io::Result<Bytes> {
    let payload = json!({
        "type": "error",
        "error": {
            "message": format!("{context}: {error}")
        }
    });
    encode_sse_json("error", &payload)
}

struct SseTranslationState<T> {
    input: Pin<Box<dyn Stream<Item = Result<Bytes, axum::Error>> + Send>>,
    scanner: SseEventScanner,
    translator: T,
    error_encoder: fn(&str, io::Error) -> io::Result<Bytes>,
    pending: VecDeque<Bytes>,
    finished_input: bool,
    closed: bool,
}
