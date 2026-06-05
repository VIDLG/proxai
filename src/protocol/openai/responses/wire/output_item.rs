use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;

use super::{
    ApplyPatchToolCall, ApplyPatchToolCallOutput, CompactionBody, CustomToolCall,
    CustomToolCallOutputResource, FileSearchToolCall, FunctionToolCall,
    FunctionToolCallOutputResource, MCPApprovalRequest, MCPListTools, MCPToolCall, OutputMessage,
    ReasoningItem, ToolSearchCall, ToolSearchOutput, WebSearchToolCall,
};
use super::{CodeInterpreterToolCall, ComputerToolCall, ComputerToolCallOutputResource};
use super::{FunctionShellCall, FunctionShellCallOutput, ImageGenToolCall, LocalShellToolCall};

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::OutputItem))]
#[serde(tag = "type", rename_all = "snake_case")]
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
    ShellCall(FunctionShellCall),
    ShellCallOutput(FunctionShellCallOutput),
    ApplyPatchCall(ApplyPatchToolCall),
    ApplyPatchCallOutput(ApplyPatchToolCallOutput),
    McpCall(MCPToolCall),
    McpListTools(MCPListTools),
    McpApprovalRequest(MCPApprovalRequest),
    CustomToolCall(CustomToolCall),
    CustomToolCallOutput(CustomToolCallOutputResource),
    ToolSearchCall(ToolSearchCall),
    ToolSearchOutput(ToolSearchOutput),
}
