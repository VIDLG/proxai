use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumDiscriminants};

use super::{
    ApplyPatchToolCall, ApplyPatchToolCallOutput, CompactionBody, CustomToolCall,
    CustomToolCallOutputResource, FileSearchToolCall, FunctionToolCall,
    FunctionToolCallOutputResource, LocalShellToolCallOutput, MCPApprovalRequest,
    MCPApprovalResponseResource, MCPListTools, MCPToolCall, OutputMessage, ReasoningItem,
    ToolSearchCall, ToolSearchOutput, WebSearchToolCall,
};
use super::{CodeInterpreterToolCall, ComputerToolCall, ComputerToolCallOutputResource};
use super::{
    FunctionShellCall, FunctionShellCallOutput, ImageGenToolCall, LocalShellToolCall, Tool,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdditionalToolsRole {
    User,
    Assistant,
    System,
    Critic,
    Discriminator,
    Developer,
    Tool,
    Unknown,
}

/// OpenAPI schema: `#/components/schemas/AdditionalTools`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdditionalTools {
    pub id: String,
    pub role: AdditionalToolsRole,
    pub tools: Vec<Tool>,
}

/// OpenAPI schema: `#/components/schemas/Program`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    pub id: String,
    pub call_id: String,
    pub code: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgramOutputStatus {
    Completed,
    Incomplete,
}

/// OpenAPI schema: `#/components/schemas/ProgramOutput`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramOutput {
    pub id: String,
    pub call_id: String,
    pub result: String,
    pub status: ProgramOutputStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AsRefStr, EnumDiscriminants)]
#[serde(tag = "type", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(
    name(OutputItemKind),
    vis(pub),
    derive(PartialOrd, Ord, Display, Serialize),
    strum(serialize_all = "snake_case"),
    serde(rename_all = "snake_case")
)]
pub enum OutputItem {
    AdditionalTools(AdditionalTools),
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
    Program(Program),
    ProgramOutput(ProgramOutput),
}

impl OutputItem {
    /// Returns the platform item ID when the wire shape carries one.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::AdditionalTools(item) => Some(&item.id),
            Self::Message(item) => Some(&item.id),
            Self::FileSearchCall(item) => Some(&item.id),
            Self::FunctionCall(item) => item.id.as_deref(),
            Self::FunctionCallOutput(item) => Some(&item.id),
            Self::WebSearchCall(item) => Some(&item.id),
            Self::ComputerCall(item) => Some(&item.id),
            Self::ComputerCallOutput(item) => Some(&item.id),
            Self::Reasoning(item) => Some(&item.id),
            Self::Compaction(item) => Some(&item.id),
            Self::ImageGenerationCall(item) => Some(&item.id),
            Self::CodeInterpreterCall(item) => Some(&item.id),
            Self::LocalShellCall(item) => Some(&item.id),
            Self::LocalShellCallOutput(item) => Some(&item.id),
            Self::ShellCall(item) => Some(&item.id),
            Self::ShellCallOutput(item) => Some(&item.id),
            Self::ApplyPatchCall(item) => Some(&item.id),
            Self::ApplyPatchCallOutput(item) => Some(&item.id),
            Self::McpCall(item) => Some(&item.id),
            Self::McpListTools(item) => Some(&item.id),
            Self::McpApprovalRequest(item) => Some(&item.id),
            Self::McpApprovalResponse(item) => Some(&item.id),
            Self::CustomToolCall(item) => item.id.as_deref(),
            Self::CustomToolCallOutput(item) => Some(&item.id),
            Self::ToolSearchCall(item) => Some(&item.id),
            Self::ToolSearchOutput(item) => Some(&item.id),
            Self::Program(item) => Some(&item.id),
            Self::ProgramOutput(item) => Some(&item.id),
        }
    }
}
