// Normalizes OpenAI Responses API request payloads before protocol parsing.
//
// Keep this narrow and explicit:
// - only inspect the top-level request `input` array;
// - move top-level system messages from `input` into `instructions`;
// - normalize Zed assistant replay messages into canonical OutputMessage shape;
// - leave nested tool schemas and unrelated payload fields untouched.
use serde_json::{Map, Value};

pub(crate) fn normalize_payload(mut value: Value) -> Value {
    let Some(object) = value.as_object_mut() else {
        return value;
    };

    let can_extract_system_messages = matches!(
        object.get("instructions"),
        None | Some(Value::Null) | Some(Value::String(_))
    );
    let system_texts = match object.get_mut("input") {
        Some(Value::Array(items)) => normalize_input_items(items, can_extract_system_messages),
        _ => Vec::new(),
    };
    if !system_texts.is_empty() {
        merge_instructions(object, system_texts.join("\n\n"));
    }

    value
}

fn normalize_input_items(items: &mut Vec<Value>, can_extract_system_messages: bool) -> Vec<String> {
    let original_items = std::mem::take(items);
    let mut system_texts = Vec::new();

    for (index, mut item) in original_items.into_iter().enumerate() {
        if can_extract_system_messages
            && is_system_message(&item)
            && let Some(text) = extract_text(item.get("content"))
        {
            system_texts.push(text);
            continue;
        }

        normalize_assistant_replay_message(&mut item, index);
        items.push(item);
    }

    system_texts
}

fn is_system_message(item: &Value) -> bool {
    item.get("role")
        .and_then(Value::as_str)
        .is_some_and(|role| role == "system")
}

fn normalize_assistant_replay_message(item: &mut Value, index: usize) {
    let Some(object) = item.as_object_mut() else {
        return;
    };
    if object.get("type").and_then(Value::as_str) != Some("message")
        || object.get("role").and_then(Value::as_str) != Some("assistant")
    {
        return;
    }

    let has_only_output_content = object
        .get("content")
        .and_then(Value::as_array)
        .is_some_and(|parts| !parts.is_empty() && parts.iter().all(is_output_message_content_part));
    if !has_only_output_content {
        return;
    }

    object
        .entry("id".to_string())
        .or_insert_with(|| Value::String(format!("msg_zed_replay_{index}")));
    object
        .entry("status".to_string())
        .or_insert_with(|| Value::String("completed".to_string()));

    if let Some(parts) = object.get_mut("content").and_then(Value::as_array_mut) {
        for part in parts {
            normalize_output_message_content_part(part);
        }
    }
}

fn is_output_message_content_part(part: &Value) -> bool {
    part.get("type")
        .and_then(Value::as_str)
        .is_some_and(|part_type| matches!(part_type, "output_text" | "refusal"))
}

fn normalize_output_message_content_part(part: &mut Value) {
    let Some(object) = part.as_object_mut() else {
        return;
    };
    if object.get("type").and_then(Value::as_str) == Some("output_text") {
        object
            .entry("annotations".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
    }
}

fn merge_instructions(normalized: &mut Map<String, Value>, extracted: String) {
    match normalized.get("instructions") {
        Some(Value::String(existing)) if !existing.trim().is_empty() => {
            normalized.insert(
                "instructions".to_string(),
                Value::String(format!("{extracted}\n\n{existing}")),
            );
        }
        Some(Value::String(_)) | None | Some(Value::Null) => {
            normalized.insert("instructions".to_string(), Value::String(extracted));
        }
        Some(_) => {}
    }
}

fn extract_text(content: Option<&Value>) -> Option<String> {
    match content? {
        Value::String(text) => Some(text.clone()).filter(|text| !text.is_empty()),
        Value::Array(parts) => {
            if parts.is_empty() {
                return None;
            }

            let mut texts = Vec::with_capacity(parts.len());
            for part in parts {
                let object = part.as_object()?;
                let part_type = object.get("type")?.as_str()?;
                if !matches!(part_type, "input_text" | "text") {
                    return None;
                }
                let text = object.get("text")?.as_str()?;
                if text.is_empty() {
                    return None;
                }
                texts.push(text);
            }

            Some(texts.join("\n"))
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod tests;
