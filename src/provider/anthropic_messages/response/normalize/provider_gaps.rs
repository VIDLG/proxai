use serde_json::{json, Map, Value};

pub(super) fn normalize_content_block(block: &mut Map<String, Value>) {
    match block.get("type").and_then(Value::as_str) {
        // Measured MiniMax Anthropic-compatible streams can emit a thinking
        // content_block_start without `signature`. Keep this provider-specific
        // repair narrow: do not infer defaults for unrelated required fields.
        Some("thinking") => {
            block
                .entry("signature".to_string())
                .or_insert_with(|| Value::String(String::new()));
        }
        // Measured Mimo Anthropic-compatible streams can emit response-side
        // tool_use blocks without the SDK-required caller discriminator. Treat
        // these as ordinary client tool calls.
        Some("tool_use") => {
            block
                .entry("caller".to_string())
                .or_insert_with(|| json!({"type": "direct"}));
        }
        _ => {}
    }
}

pub(super) fn normalize_server_tool_usage(usage: &mut Map<String, Value>) {
    let Some(server_tool_use) = usage
        .get_mut("server_tool_use")
        .and_then(Value::as_object_mut)
    else {
        return;
    };

    // Measured GLM 5.1 Anthropic-compatible streams can emit
    // `server_tool_use: {"web_search_requests": 0}` without the sibling
    // `web_fetch_requests`. The official SDK requires both counters when the
    // object is present, so fill absent counters with zero.
    server_tool_use
        .entry("web_fetch_requests".to_string())
        .or_insert_with(|| json!(0));
    server_tool_use
        .entry("web_search_requests".to_string())
        .or_insert_with(|| json!(0));
}
