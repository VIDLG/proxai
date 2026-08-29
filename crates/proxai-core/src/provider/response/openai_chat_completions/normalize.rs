use serde_json::Value;

pub(crate) fn normalize_response_payload(mut payload: Value) -> Value {
    normalize_service_tier(&mut payload);
    payload
}

pub(crate) fn normalize_stream_event_payload(mut payload: Value) -> Value {
    normalize_service_tier(&mut payload);
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

fn normalize_service_tier(payload: &mut Value) {
    let Some(service_tier) = payload.get_mut("service_tier") else {
        return;
    };
    // MiniMax-M3 leaked Anthropic's `standard` response-tier spelling into an
    // OpenAI Chat chunk (diagnostic 1784191897-1784191896360, 2026-07-16).
    // OpenAI calls the equivalent standard pricing/performance tier `default`.
    if service_tier.as_str() == Some("standard") {
        *service_tier = Value::String("default".to_string());
    }
}
