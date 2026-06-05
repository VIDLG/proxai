use axum::body::Bytes;
use serde_json::{Map, Value};

mod event_payload;
mod message;
mod provider_gaps;
mod response_shape;
mod stream;

use message::normalize_message_object;

pub(crate) use event_payload::normalize_stream_event_payload;
pub(super) use stream::normalize_sse_stream;

pub(crate) fn normalize_message_payload(mut payload: Value) -> Value {
    let Some(object) = payload
        .as_object_mut()
        .filter(|object| is_message_like(object))
    else {
        return payload;
    };

    normalize_message_object(object);
    payload
}

pub(crate) fn normalize_message_body_bytes(body: &[u8]) -> serde_json::Result<Bytes> {
    let payload = serde_json::from_slice::<Value>(body)?;
    if !payload.as_object().is_some_and(is_message_like) {
        return Ok(Bytes::copy_from_slice(body));
    }
    let normalized = normalize_message_payload(payload);
    Ok(Bytes::from(serde_json::to_vec(&normalized)?))
}

fn is_message_like(object: &Map<String, Value>) -> bool {
    object.contains_key("id")
        && object.get("role").and_then(Value::as_str) == Some("assistant")
        && object.contains_key("model")
        && object.contains_key("content")
        && object.contains_key("usage")
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
