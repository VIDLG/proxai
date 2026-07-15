use serde_json::Value;

#[derive(Debug, Default)]
pub(super) struct SanitizedInputFields {
    pub(super) status_removed: usize,
    pub(super) reasoning_content_removed: usize,
}

/// Remove output-only fields from replayed Responses input items.
///
/// Responses clients can replay output items as later request input. Strict
/// compatible upstreams reject `status` on message/reasoning input items and
/// non-empty `content` on reasoning input items, while the same fields remain
/// valid on other item kinds and at the request root.
pub(super) fn sanitize_provider_payload(mut payload: Value) -> (Value, SanitizedInputFields) {
    let sanitized = sanitize_response_output_fields_from_input(&mut payload);
    (payload, sanitized)
}

fn sanitize_response_output_fields_from_input(payload: &mut Value) -> SanitizedInputFields {
    let Some(items) = payload.get_mut("input").and_then(Value::as_array_mut) else {
        return SanitizedInputFields::default();
    };

    let mut sanitized = SanitizedInputFields::default();
    for item in items {
        let Some(object) = item.as_object_mut() else {
            continue;
        };
        let Some(item_type) = object.get("type").and_then(Value::as_str) else {
            continue;
        };
        let is_message_or_reasoning = matches!(item_type, "message" | "reasoning");
        let is_reasoning = item_type == "reasoning";
        if is_message_or_reasoning && object.remove("status").is_some() {
            sanitized.status_removed += 1;
        }
        if is_reasoning && object.remove("content").is_some() {
            sanitized.reasoning_content_removed += 1;
        }
    }
    sanitized
}
