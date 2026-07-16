use serde_json::Value;

pub(crate) fn normalize_stream_event_payload(mut payload: Value) -> Value {
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
