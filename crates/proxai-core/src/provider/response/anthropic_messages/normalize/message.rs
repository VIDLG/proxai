use serde_json::{Map, Value};

use super::{provider_gaps, response_shape};

pub(super) fn normalize_message_object(object: &mut Map<String, Value>) {
    object
        .entry("type".to_string())
        .or_insert_with(|| Value::String("message".to_string()));

    response_shape::normalize_message_status_fields(object);

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
