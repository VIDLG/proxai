use serde_json::{Map, Value};

pub(crate) fn normalize_response_payload(mut payload: Value) -> Value {
    if let Some(response) = payload.as_object_mut() {
        normalize_usage(response);
    }
    payload
}

pub(crate) fn normalize_stream_event_payload(mut payload: Value) -> Value {
    if let Some(response) = payload.get_mut("response").and_then(Value::as_object_mut) {
        normalize_usage(response);
    }
    payload
}

fn normalize_usage(response: &mut Map<String, Value>) {
    let Some(details) = response
        .get_mut("usage")
        .and_then(Value::as_object_mut)
        .and_then(|usage| usage.get_mut("input_tokens_details"))
        .and_then(Value::as_object_mut)
    else {
        return;
    };

    // OpenAI added the required `cache_write_tokens` counter in the 2026-07
    // schema. Older Responses-compatible providers emit the otherwise complete
    // usage object without it; compatible mode uses zero as the neutral fallback
    // when the provider did not report a cache-write count.
    details
        .entry("cache_write_tokens".to_string())
        .or_insert_with(|| Value::from(0));
}
