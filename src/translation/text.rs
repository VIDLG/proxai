//! General-purpose text and JSON helpers shared across translation pairs.

use serde_json::Value;

/// Join text parts into a single string, trimming each part and separating
/// non-empty parts with a blank line. Returns `None` when the result would
/// be empty.
pub(crate) fn join_text_parts(parts: Vec<String>) -> Option<String> {
    let text = parts
        .into_iter()
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    (!text.is_empty()).then_some(text)
}

/// Parse a JSON tool input string into a `Value`. Falls back to the raw
/// string wrapped in `Value::String` when the input is not valid JSON,
/// so tool-call argument streams that arrive as plain text still carry
/// through without dropping data.
pub(crate) fn parse_json_or_string(input: &str) -> Value {
    serde_json::from_str::<Value>(input).unwrap_or_else(|_| Value::String(input.to_string()))
}
