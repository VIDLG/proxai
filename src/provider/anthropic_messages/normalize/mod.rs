use axum::body::Bytes;
use serde_json::{Map, Value};

mod event_payload;
mod provider_gaps;
mod response_shape;
mod stream;

pub(crate) use event_payload::normalize_stream_event_payload;
pub(super) use stream::normalize_sse_stream;

pub(crate) fn normalize_message_payload(mut payload: Value) -> Value {
    if let Some(object) = payload
        .as_object_mut()
        .filter(|object| is_message_like(object))
    {
        normalize_message_object(object);
    }
    payload
}

pub(super) fn normalize_body_bytes(body: &[u8]) -> serde_json::Result<Bytes> {
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

fn normalize_message_object(object: &mut Map<String, Value>) {
    object
        .entry("type".to_string())
        .or_insert_with(|| Value::String("message".to_string()));

    response_shape::normalize_message_object(object);

    if let Some(content) = object.get_mut("content").and_then(Value::as_array_mut) {
        for block in content {
            if let Some(block) = block.as_object_mut() {
                response_shape::normalize_content_block(block);
                provider_gaps::normalize_content_block(block);
            }
        }
    }

    if let Some(usage) = object.get_mut("usage").and_then(Value::as_object_mut) {
        response_shape::normalize_message_usage(usage);
        provider_gaps::normalize_server_tool_usage(usage);
    }
}

pub(super) fn insert_nulls(object: &mut Map<String, Value>, keys: &[&str]) {
    for key in keys {
        object.entry((*key).to_string()).or_insert(Value::Null);
    }
}

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod tests;
