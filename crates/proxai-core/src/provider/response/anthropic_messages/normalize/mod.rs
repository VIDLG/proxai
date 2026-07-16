use serde_json::{Map, Value};

mod message;
mod provider_gaps;
mod response_shape;
mod stream_event;

use message::normalize_message_object;

pub(crate) use stream_event::normalize_stream_event_payload;

pub(crate) fn normalize_response_payload(mut payload: Value) -> Value {
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
