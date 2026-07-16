// Normalizes OpenAI Responses API request payloads before protocol parsing.
//
// Keep this narrow and explicit:
// - only inspect the top-level request `input` array;
// - move top-level system messages from `input` into `instructions`;
// - complete Zed's compact assistant replay messages and id-less reasoning
//   history into standard Responses items;
// - supply the official default detail for Zed message images;
// - complete compact top-level function tool definitions with the required
//   nullable `strict` carrier field;
// - leave nested tool schemas and unrelated payload fields untouched.
use serde_json::{Map, Value};

pub(crate) fn normalize_payload(mut value: Value) -> Value {
    let Some(object) = value.as_object_mut() else {
        return value;
    };

    normalize_function_tools(object);

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

fn normalize_function_tools(object: &mut Map<String, Value>) {
    let Some(tools) = object.get_mut("tools").and_then(Value::as_array_mut) else {
        return;
    };

    for tool in tools {
        let Some(tool) = tool.as_object_mut() else {
            continue;
        };
        if tool.get("type").and_then(Value::as_str) == Some("function") {
            tool.entry("strict".to_string()).or_insert(Value::Null);
        }
    }
}

fn normalize_input_items(items: &mut Vec<Value>, can_extract_system_messages: bool) -> Vec<String> {
    let original_items = std::mem::take(items);
    let mut system_texts = Vec::new();

    for (input_index, mut item) in original_items.into_iter().enumerate() {
        if can_extract_system_messages
            && is_system_message(&item)
            && let Some(text) = extract_text(item.get("content"))
        {
            system_texts.push(text);
            continue;
        }

        normalize_zed_message_images(&mut item);
        normalize_zed_assistant_replay(&mut item, input_index);
        normalize_zed_reasoning_replay(&mut item, input_index);
        items.push(item);
    }

    system_texts
}

fn normalize_zed_message_images(item: &mut Value) {
    let Some(message) = item.as_object_mut() else {
        return;
    };
    if message.get("type").and_then(Value::as_str) != Some("message") {
        return;
    }
    let Some(content) = message.get_mut("content").and_then(Value::as_array_mut) else {
        return;
    };

    for part in content {
        let Some(image) = part.as_object_mut() else {
            continue;
        };
        if image.get("type").and_then(Value::as_str) == Some("input_image") {
            // Zed's message image model has no `detail` field, while the
            // official `InputImageContent` requires it and defines `auto` as the
            // default. Function-call outputs use a different official image
            // parameter where omission is valid, so only message content is
            // normalized here.
            image
                .entry("detail".to_string())
                .or_insert_with(|| Value::String("auto".to_string()));
        }
    }
}

fn normalize_zed_assistant_replay(item: &mut Value, input_index: usize) {
    let Some(message) = item.as_object_mut() else {
        return;
    };
    if message.get("role").and_then(Value::as_str) != Some("assistant")
        || message.contains_key("id")
        || message.contains_key("status")
        || message
            .get("type")
            .is_some_and(|message_type| message_type.as_str() != Some("message"))
    {
        return;
    }

    let Some(content) = message.get_mut("content").and_then(Value::as_array_mut) else {
        return;
    };
    if !content.iter().all(is_assistant_output_content) {
        return;
    }

    for part in content {
        if part.get("type").and_then(Value::as_str) == Some("output_text")
            && let Some(part) = part.as_object_mut()
        {
            // `OutputTextContent` requires both arrays in the official Responses
            // schema. Only Zed's recognized compact replay gets these lossless
            // defaults; malformed or unknown content remains a protocol error.
            part.entry("annotations".to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
            part.entry("logprobs".to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
        }
    }

    // Zed replays completed assistant history without the item envelope that
    // Responses output messages require. Use a deterministic local id because
    // these ids are only needed to satisfy the request schema before projection.
    message.insert("type".to_string(), Value::String("message".to_string()));
    message.insert(
        "id".to_string(),
        Value::String(format!("msg_zed_replay_{input_index}")),
    );
    message.insert("status".to_string(), Value::String("completed".to_string()));
}

fn normalize_zed_reasoning_replay(item: &mut Value, input_index: usize) {
    let Some(reasoning) = item.as_object_mut() else {
        return;
    };
    if reasoning.get("type").and_then(Value::as_str) != Some("reasoning")
        || reasoning.contains_key("id")
        || !reasoning.get("summary").is_some_and(Value::is_array)
    {
        return;
    }

    // The official `ReasoningItem` requires `id`, but Zed's
    // `ResponseReasoningInputItem` intentionally models it as optional and
    // replays id-less summary items (`contrib/zed`, commit `78c889c21d7`). The
    // generated id is deterministic and exists only to restore the official
    // request envelope before routing and projection.
    reasoning.insert(
        "id".to_string(),
        Value::String(format!("rs_zed_replay_{input_index}")),
    );
}

fn is_assistant_output_content(part: &Value) -> bool {
    matches!(
        part.get("type").and_then(Value::as_str),
        Some("output_text" | "refusal")
    )
}

fn is_system_message(item: &Value) -> bool {
    item.get("role")
        .and_then(Value::as_str)
        .is_some_and(|role| role == "system")
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

#[cfg(test)]
#[path = "normalize_regression_tests.rs"]
mod regression_tests;
