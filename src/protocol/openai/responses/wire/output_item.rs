use serde::{Deserialize, Serialize};
use strum::AsRefStr;

use super::{
    ApplyPatchToolCall, ApplyPatchToolCallOutput, CompactionBody, CustomToolCall,
    CustomToolCallOutputResource, FileSearchToolCall, FunctionToolCall,
    FunctionToolCallOutputResource, LocalShellToolCallOutput, MCPApprovalRequest,
    MCPApprovalResponseResource, MCPListTools, MCPToolCall, OutputMessage, ReasoningItem,
    ToolSearchCall, ToolSearchOutput, WebSearchToolCall,
};
use super::{CodeInterpreterToolCall, ComputerToolCall, ComputerToolCallOutputResource};
use super::{FunctionShellCall, FunctionShellCallOutput, ImageGenToolCall, LocalShellToolCall};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AsRefStr)]
#[serde(tag = "type", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OutputItem {
    Message(OutputMessage),
    FileSearchCall(FileSearchToolCall),
    FunctionCall(FunctionToolCall),
    FunctionCallOutput(FunctionToolCallOutputResource),
    WebSearchCall(WebSearchToolCall),
    ComputerCall(ComputerToolCall),
    ComputerCallOutput(ComputerToolCallOutputResource),
    Reasoning(ReasoningItem),
    Compaction(CompactionBody),
    ImageGenerationCall(ImageGenToolCall),
    CodeInterpreterCall(CodeInterpreterToolCall),
    LocalShellCall(LocalShellToolCall),
    LocalShellCallOutput(LocalShellToolCallOutput),
    ShellCall(FunctionShellCall),
    ShellCallOutput(FunctionShellCallOutput),
    ApplyPatchCall(ApplyPatchToolCall),
    ApplyPatchCallOutput(ApplyPatchToolCallOutput),
    McpCall(MCPToolCall),
    McpListTools(MCPListTools),
    McpApprovalRequest(MCPApprovalRequest),
    McpApprovalResponse(MCPApprovalResponseResource),
    CustomToolCall(CustomToolCall),
    CustomToolCallOutput(CustomToolCallOutputResource),
    ToolSearchCall(ToolSearchCall),
    ToolSearchOutput(ToolSearchOutput),
}
