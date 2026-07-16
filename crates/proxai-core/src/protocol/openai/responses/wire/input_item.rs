use crate::protocol::OptionalNullable;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

use super::{
    ApplyPatchToolCallItemParam, ApplyPatchToolCallOutputItemParam, CodeInterpreterToolCall,
    CompactionSummaryItemParam, ComputerCallOutputItemParam, ComputerToolCall, CustomToolCall,
    CustomToolCallOutput, EasyInputMessage, FileSearchToolCall, FunctionCallOutputItemParam,
    FunctionShellCallItemParam, FunctionShellCallOutputItemParam, FunctionToolCall,
    ImageGenToolCall, InputMessage, LocalShellToolCall, LocalShellToolCallOutput,
    MCPApprovalRequest, MCPApprovalResponse, MCPListTools, MCPToolCall, OutputMessage,
    ReasoningItem, Tool, ToolSearchCallItemParam, ToolSearchOutputItemParam, WebSearchToolCall,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemReferenceType {
    ItemReference,
}

/// OpenAPI schema: `#/components/schemas/ItemReferenceParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemReferenceParam {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub r#type: OptionalNullable<ItemReferenceType>,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageItem {
    Output(OutputMessage),
    Input(InputMessage),
}

/// OpenAPI schema: `#/components/schemas/AdditionalToolsItemParam`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum AdditionalToolsItemRole {
    Developer,
}

/// OpenAPI schema: `#/components/schemas/AdditionalToolsItemParam`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdditionalToolsItemParam {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub id: OptionalNullable<String>,
    pub role: AdditionalToolsItemRole,
    pub tools: Vec<Tool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AsRefStr)]
#[serde(tag = "type", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Item {
    AdditionalTools(AdditionalToolsItemParam),
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputParam {
    Text(String),
    Items(Vec<InputItem>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputItem {
    ItemReference(ItemReferenceParam),
    Item(Item),
    EasyMessage(EasyInputMessage),
}

#[cfg(test)]
#[path = "input_item_tests.rs"]
mod tests;
