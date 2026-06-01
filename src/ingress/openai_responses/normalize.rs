// Normalizes OpenAI Responses API request payloads for upstreams that reject
// `role: "system"` entries inside `input`.
//
// The traversal is intentionally conservative:
// - recursively normalize nested arrays and objects;
// - when an object has an `input` array, remove system-role items from it;
// - extract text from those removed system messages;
// - prepend the extracted text to `instructions`, preserving any existing text;
// - leave all unrelated fields and non-JSON shapes unchanged.
use serde_json::{Map, Value};

pub(crate) fn normalize_payload(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_payload).collect()),
        Value::Object(object) => normalize_object(object),
        other => other,
    }
}

fn normalize_object(object: Map<String, Value>) -> Value {
    let mut normalized = Map::new();
    let mut input_value = None;

    for (key, value) in object {
        if key == "input" {
            input_value = Some(value);
        } else {
            normalized.insert(key, normalize_payload(value));
        }
    }

    match input_value {
        Some(Value::Array(items)) => {
            let system_texts = normalize_input(items, &mut normalized);
            if !system_texts.is_empty() {
                merge_instructions(&mut normalized, system_texts.join("\n\n"));
            }
        }
        Some(input) => {
            normalized.insert("input".to_string(), normalize_payload(input));
        }
        None => {}
    }

    Value::Object(normalized)
}

fn normalize_input(items: Vec<Value>, normalized: &mut Map<String, Value>) -> Vec<String> {
    let mut input = Vec::with_capacity(items.len());
    let mut system_texts = Vec::new();

    for item in items {
        let item = normalize_payload(item);
        if item
            .get("role")
            .and_then(Value::as_str)
            .is_some_and(|role| role == "system")
        {
            if let Some(text) = extract_text(item.get("content")) {
                system_texts.push(text);
            }
        } else {
            input.push(item);
        }
    }

    normalized.insert("input".to_string(), Value::Array(input));
    system_texts
}

fn merge_instructions(normalized: &mut Map<String, Value>, extracted: String) {
    match normalized.get("instructions") {
        Some(Value::String(existing)) if !existing.trim().is_empty() => {
            normalized.insert(
                "instructions".to_string(),
                Value::String(format!("{extracted}\n\n{existing}")),
            );
        }
        _ => {
            normalized.insert("instructions".to_string(), Value::String(extracted));
        }
    }
}

fn extract_text(content: Option<&Value>) -> Option<String> {
    match content? {
        Value::String(text) => Some(text.clone()).filter(|text| !text.is_empty()),
        Value::Array(parts) => {
            let text = parts
                .iter()
                .filter_map(|part| {
                    let object = part.as_object()?;
                    let part_type = object.get("type")?.as_str()?;
                    if !matches!(part_type, "input_text" | "text") {
                        return None;
                    }
                    object.get("text")?.as_str()
                })
                .collect::<Vec<_>>()
                .join("\n");
            Some(text).filter(|text| !text.is_empty())
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod tests;
