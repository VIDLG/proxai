use serde_json::{Map, Value};

mod event_payload;
mod message;
mod provider_gaps;
mod response_shape;

use message::normalize_message_object;

pub(crate) use event_payload::normalize_stream_event_payload;

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

fn is_message_like(object: &Map<String, Value>) -> bool {
    object.contains_key("id")
        && object.get("role").and_then(Value::as_str) == Some("assistant")
        && object.contains_key("model")
        && object.contains_key("content")
        && object.contains_key("usage")
}

#[cfg(test)]
mod tests;
