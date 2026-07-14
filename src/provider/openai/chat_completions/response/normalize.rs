use async_stream::try_stream;
use axum::body::Bytes;
use futures_util::{Stream, StreamExt};
use serde_json::Value;

use crate::http_support::{ByteStream, ByteStreamError};
use crate::sse::sse_event_stream;

pub(super) fn normalize_stream_event_payload(mut payload: Value) -> Value {
    let Some(choices) = payload.get_mut("choices").and_then(Value::as_array_mut) else {
        return payload;
    };

    // MiniMax-M3 emitted intermediate Chat chunks without this OpenAPI
    // required-nullable field (local Zed/proxai request `kjpuu8`, 2026-07-14).
    // Keep the wire struct strict and repair only this provider-compatibility
    // omission at the response boundary: omitted means JSON `null`, not an
    // invented terminal finish reason.
    for choice in choices {
        if let Some(choice) = choice.as_object_mut() {
            choice
                .entry("finish_reason".to_string())
                .or_insert(Value::Null);
        }
    }

    payload
}

pub(super) fn normalize_sse_stream<S, E>(input: S) -> ByteStream
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
    E: Into<ByteStreamError> + Send + 'static,
{
    let stream = try_stream! {
        let mut events = Box::pin(sse_event_stream(input));

        while let Some(event) = events.next().await {
            let event = event?;
            if event.is_done_sentinel() {
                yield crate::sse::done_sentinel_bytes();
                continue;
            }

            let payload = normalize_stream_event_payload(event.payload_with_type()?);
            yield event.encode_json_payload(&payload)?;
        }
    };

    Box::pin(stream)
}

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod tests;
