use crate::protocol::openai_responses as responses;

// Anthropic Messages requires `max_tokens`, while Responses allows it to be
// omitted. This is a proxai compatibility fallback for OpenAI-compatible clients
// that omit output limits; it is not an upstream protocol default.
pub(super) const DEFAULT_MAX_TOKENS: u32 = 4096;

pub(super) fn json_number_from_f32(value: f32) -> Option<serde_json::Number> {
    serde_json::Number::from_f64(value as f64)
}

pub(super) fn item_discriminant(item: &responses::Item) -> &'static str {
    match item {
        responses::Item::Message(_) => "message",
        responses::Item::FileSearchCall(_) => "file_search_call",
        responses::Item::ComputerCall(_) => "computer_call",
        responses::Item::ComputerCallOutput(_) => "computer_call_output",
        responses::Item::WebSearchCall(_) => "web_search_call",
        responses::Item::FunctionCall(_) => "function_call",
        responses::Item::FunctionCallOutput(_) => "function_call_output",
        responses::Item::ToolSearchCall(_) => "tool_search_call",
        responses::Item::ToolSearchOutput(_) => "tool_search_output",
        responses::Item::Reasoning(_) => "reasoning",
        responses::Item::Compaction(_) => "compaction",
        responses::Item::ImageGenerationCall(_) => "image_generation_call",
        responses::Item::CodeInterpreterCall(_) => "code_interpreter_call",
        responses::Item::LocalShellCall(_) => "local_shell_call",
        responses::Item::LocalShellCallOutput(_) => "local_shell_call_output",
        responses::Item::ShellCall(_) => "shell_call",
        responses::Item::ShellCallOutput(_) => "shell_call_output",
        responses::Item::ApplyPatchCall(_) => "apply_patch_call",
        responses::Item::ApplyPatchCallOutput(_) => "apply_patch_call_output",
        responses::Item::McpListTools(_) => "mcp_list_tools",
        responses::Item::McpApprovalRequest(_) => "mcp_approval_request",
        responses::Item::McpApprovalResponse(_) => "mcp_approval_response",
        responses::Item::McpCall(_) => "mcp_call",
        responses::Item::CustomToolCallOutput(_) => "custom_tool_call_output",
        responses::Item::CustomToolCall(_) => "custom_tool_call",
    }
}

pub(super) fn tool_discriminant(tool: &responses::Tool) -> &'static str {
    match tool {
        responses::Tool::Function(_) => "function",
        responses::Tool::FileSearch(_) => "file_search",
        responses::Tool::ComputerUsePreview(_) => "computer_use_preview",
        responses::Tool::WebSearch(_) => "web_search",
        responses::Tool::WebSearch20250826(_) => "web_search_20250826",
        responses::Tool::Mcp(_) => "mcp",
        responses::Tool::CodeInterpreter(_) => "code_interpreter",
        responses::Tool::ImageGeneration(_) => "image_generation",
        responses::Tool::LocalShell => "local_shell",
        responses::Tool::Shell(_) => "shell",
        responses::Tool::Custom(_) => "custom",
        responses::Tool::Computer(_) => "computer",
        responses::Tool::Namespace(_) => "namespace",
        responses::Tool::ToolSearch(_) => "tool_search",
        responses::Tool::WebSearchPreview(_) => "web_search_preview",
        responses::Tool::WebSearchPreview20250311(_) => "web_search_preview_20250311",
        responses::Tool::ApplyPatch => "apply_patch",
    }
}
