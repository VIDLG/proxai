use serde_json::{Map, Value};

use super::{message::normalize_message_object, provider_gaps, response_shape};

pub(crate) fn normalize_stream_event_payload(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        match object.get("type").and_then(Value::as_str) {
            Some("message_start") => normalize_message_start(object),
            Some("content_block_start") => normalize_content_block_start(object),
            Some("message_delta") => normalize_message_delta_event(object),
            _ => {}
        }
    }
    payload
}

fn normalize_message_start(object: &mut Map<String, Value>) {
    if let Some(message) = object.get_mut("message").and_then(Value::as_object_mut) {
        normalize_message_object(message);
    }
}

fn normalize_content_block_start(object: &mut Map<String, Value>) {
    let Some(block) = object
        .get_mut("content_block")
        .and_then(Value::as_object_mut)
    else {
        return;
    };

    response_shape::normalize_content_block(block);
    provider_gaps::normalize_content_block(block);
}

fn normalize_message_delta_event(object: &mut Map<String, Value>) {
    if let Some(delta) = object.get_mut("delta").and_then(Value::as_object_mut) {
        response_shape::normalize_message_status_fields(delta);
    }

    if let Some(usage) = object.get_mut("usage").and_then(Value::as_object_mut) {
        response_shape::normalize_message_delta_usage(usage);
        provider_gaps::normalize_server_tool_usage(usage);
    }
}
