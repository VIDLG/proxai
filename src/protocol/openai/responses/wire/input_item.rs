use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;

use super::{
    ApplyPatchToolCallItemParam, ApplyPatchToolCallOutputItemParam, CodeInterpreterToolCall,
    CompactionSummaryItemParam, ComputerCallOutputItemParam, ComputerToolCall, CustomToolCall,
    CustomToolCallOutput, EasyInputMessage, FileSearchToolCall, FunctionCallOutputItemParam,
    FunctionShellCallItemParam, FunctionShellCallOutputItemParam, FunctionToolCall,
    ImageGenToolCall, InputMessage, LocalShellToolCall, LocalShellToolCallOutput,
    MCPApprovalRequest, MCPApprovalResponse, MCPListTools, MCPToolCall, OutputMessage,
    ReasoningItem, ToolSearchCallItemParam, ToolSearchOutputItemParam, WebSearchToolCall,
};

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ItemReferenceType))]
pub enum ItemReferenceType {
    ItemReference,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ItemReference))]
pub struct ItemReference {
    pub r#type: Option<ItemReferenceType>,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::MessageItem))]
#[serde(untagged)]
pub enum MessageItem {
    Output(OutputMessage),
    Input(InputMessage),
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::Item))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Item {
    Message(MessageItem),
    FileSearchCall(FileSearchToolCall),
    ComputerCall(ComputerToolCall),
    ComputerCallOutput(ComputerCallOutputItemParam),
    WebSearchCall(WebSearchToolCall),
    FunctionCall(FunctionToolCall),
    FunctionCallOutput(FunctionCallOutputItemParam),
    ToolSearchCall(ToolSearchCallItemParam),
    ToolSearchOutput(ToolSearchOutputItemParam),
    Reasoning(ReasoningItem),
    Compaction(CompactionSummaryItemParam),
    ImageGenerationCall(ImageGenToolCall),
    CodeInterpreterCall(CodeInterpreterToolCall),
    LocalShellCall(LocalShellToolCall),
    LocalShellCallOutput(LocalShellToolCallOutput),
    ShellCall(FunctionShellCallItemParam),
    ShellCallOutput(FunctionShellCallOutputItemParam),
    ApplyPatchCall(ApplyPatchToolCallItemParam),
    ApplyPatchCallOutput(ApplyPatchToolCallOutputItemParam),
    McpListTools(MCPListTools),
    McpApprovalRequest(MCPApprovalRequest),
    McpApprovalResponse(MCPApprovalResponse),
    McpCall(MCPToolCall),
    CustomToolCallOutput(CustomToolCallOutput),
    CustomToolCall(CustomToolCall),
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::InputParam))]
#[serde(untagged)]
pub enum InputParam {
    Text(String),
    Items(Vec<InputItem>),
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::InputItem))]
#[serde(untagged)]
pub enum InputItem {
    ItemReference(ItemReference),
    Item(Item),
    EasyMessage(EasyInputMessage),
}
