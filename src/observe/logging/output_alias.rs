pub(crate) fn compact_output_item_kind(kind: &str) -> &str {
    match kind {
        "message" => "m",
        "function_call" => "fn",
        "custom_tool_call" => "ct",
        "mcp_call" => "mcp",
        "web_search_call" => "web",
        "file_search_call" => "file",
        "computer_call" => "comp",
        "code_interpreter_call" => "code",
        "image_generation_call" => "img",
        "reasoning" => "rsn",
        "tool_use" => "tu",
        "server_tool_use" => "stu",
        "text" => "txt",
        "thinking" => "think",
        // Stream-level transport events: hidden from human output
        s if s.starts_with("stream_") => "",
        other => other,
    }
}
