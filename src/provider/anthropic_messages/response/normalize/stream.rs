use async_stream::try_stream;
use axum::body::Bytes;
use futures_util::{Stream, StreamExt};

use crate::http_support::{ByteStream, ByteStreamError};
use crate::sse::{SseError, SseResult, encode_sse_json, sse_event_stream};

use super::normalize_stream_event_payload;

pub(in super::super) fn normalize_sse_stream<S, E>(input: S) -> ByteStream
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
    E: Into<ByteStreamError> + Send + 'static,
{
    let stream = try_stream! {
        let mut events = Box::pin(sse_event_stream(input));

        while let Some(event) = events.next().await {
            let event = event?;
            match normalize_sse_event(&event) {
                Ok(chunk) => yield chunk,
                Err(_) => yield encode_raw_sse_event(&event),
            }
        }
    };
    Box::pin(stream)
}

fn normalize_sse_event(event: &crate::sse::SseEvent) -> SseResult<Bytes> {
    let payload = normalize_stream_event_payload(event.payload_with_type()?);
    let event_type = payload
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or(SseError::MissingField("type"))?;
    Ok(encode_sse_json(event_type, &payload)?)
}

fn encode_raw_sse_event(event: &crate::sse::SseEvent) -> Bytes {
    let mut encoded = String::new();
    if !event.is_default_event_type() {
        encoded.push_str("event: ");
        encoded.push_str(&event.event_type);
        encoded.push('\n');
    }
    encoded.push_str("data: ");
    encoded.push_str(&event.data);
    encoded.push_str("\n\n");
    Bytes::from(encoded)
}
