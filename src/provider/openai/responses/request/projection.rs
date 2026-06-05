use serde_json::Value;

use crate::protocol::openai_responses::RequestProjection;

pub(crate) fn project_payload(payload: &Value) -> Result<RequestProjection, serde_json::Error> {
    let adapted = adapt_payload_for_projection(payload);
    RequestProjection::from_payload(&adapted)
}

pub(super) fn adapt_payload_for_projection(payload: &Value) -> Value {
    let mut payload = payload.clone();

    if let Some(text) = payload.get_mut("text").and_then(Value::as_object_mut) {
        text.entry("format")
            .or_insert_with(|| serde_json::json!({ "type": "text" }));
    }

    if let Some(input) = payload.get_mut("input").and_then(Value::as_array_mut) {
        for item in input {
            let Some(object) = item.as_object_mut() else {
                continue;
            };
            if object.get("type").and_then(Value::as_str) != Some("message") {
                continue;
            }

            let role = object
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let Some(content) = object.get_mut("content").and_then(Value::as_array_mut) else {
                continue;
            };
            for part in content {
                let Some(part_type) = part.get("type").and_then(Value::as_str) else {
                    continue;
                };

                match part_type {
                    "input_text" | "text" | "input_file" => {}
                    "input_image" => {
                        if let Some(object) = part.as_object_mut() {
                            object
                                .entry("detail".to_string())
                                .or_insert_with(|| Value::String("auto".to_string()));
                        }
                    }
                    "output_text" if role == "assistant" => {
                        if let Some(object) = part.as_object_mut() {
                            object.insert(
                                "type".to_string(),
                                Value::String("input_text".to_string()),
                            );
                            object.remove("annotations");
                            object.remove("logprobs");
                        }
                    }
                    kind => {
                        let placeholder = match kind {
                            "input_audio" => "[audio omitted for request hint extraction]",
                            _ => "[content omitted for request hint extraction]",
                        };
                        *part = serde_json::json!({
                            "type": "input_text",
                            "text": placeholder,
                        });
                    }
                }
            }
        }
    }

    payload
}

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
