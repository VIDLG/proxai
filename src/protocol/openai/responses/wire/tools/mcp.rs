use async_openai::types::mcp as openai_mcp;
use async_openai::types::responses as openai;
use serde_json::Value;
use structural_convert::StructuralConvert;
use strum::Display;

// ============================================================
// Tool Choice
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ToolChoiceMCP))]
pub struct ToolChoiceMCP {
    pub name: String,
    pub server_label: String,
}

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai_mcp::MCPToolFilter))]
pub struct MCPToolFilter {
    pub read_only: Option<bool>,
    pub tool_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai_mcp::MCPToolAllowedTools))]
pub enum MCPToolAllowedTools {
    List(Vec<String>),
    Filter(MCPToolFilter),
}

#[allow(
    clippy::enum_variant_names,
    reason = "Mirrors upstream MCP connector identifiers."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai_mcp::McpToolConnectorId))]
#[strum(serialize_all = "snake_case")]
pub enum McpToolConnectorId {
    ConnectorDropbox,
    ConnectorGmail,
    ConnectorGooglecalendar,
    ConnectorGoogledrive,
    ConnectorMicrosoftteams,
    ConnectorOutlookcalendar,
    ConnectorOutlookemail,
    ConnectorSharepoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai_mcp::MCPToolApprovalSetting))]
#[strum(serialize_all = "lowercase")]
pub enum MCPToolApprovalSetting {
    Always,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai_mcp::MCPToolApprovalFilter))]
pub struct MCPToolApprovalFilter {
    pub always: Option<MCPToolFilter>,
    pub never: Option<MCPToolFilter>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai_mcp::MCPToolRequireApproval))]
pub enum MCPToolRequireApproval {
    Filter(MCPToolApprovalFilter),
    ApprovalSetting(MCPToolApprovalSetting),
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai_mcp::MCPTool))]
pub struct MCPTool {
    pub server_label: String,
    pub allowed_tools: Option<MCPToolAllowedTools>,
    pub authorization: Option<String>,
    pub connector_id: Option<McpToolConnectorId>,
    pub headers: Option<Value>,
    pub require_approval: Option<MCPToolRequireApproval>,
    pub server_description: Option<String>,
    pub server_url: Option<String>,
    pub defer_loading: Option<bool>,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::MCPApprovalResponse))]
pub struct MCPApprovalResponse {
    pub approval_request_id: String,
    pub approve: bool,
    pub id: Option<String>,
    pub reason: Option<String>,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::MCPApprovalRequest))]
pub struct MCPApprovalRequest {
    pub arguments: String,
    pub id: String,
    pub name: String,
    pub server_label: String,
}

// ============================================================
// MCP List Tools Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai_mcp::MCPListToolsTool))]
pub struct MCPListToolsTool {
    pub input_schema: Value,
    pub name: String,
    pub annotations: Option<Value>,
    pub description: Option<String>,
}

// ============================================================
// MCP List Tools Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::MCPListTools))]
pub struct MCPListTools {
    pub id: String,
    pub server_label: String,
    pub tools: Vec<MCPListToolsTool>,
    pub error: Option<String>,
}

// ============================================================
// MCP Call Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::MCPToolCallStatus))]
#[strum(serialize_all = "snake_case")]
pub enum MCPToolCallStatus {
    InProgress,
    Completed,
    Incomplete,
    Calling,
    Failed,
}

// ============================================================
// MCP Call Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::MCPToolCall))]
pub struct MCPToolCall {
    pub arguments: String,
    pub id: String,
    pub name: String,
    pub server_label: String,
    pub approval_request_id: Option<String>,
    pub error: Option<String>,
    pub output: Option<String>,
    pub status: Option<MCPToolCallStatus>,
}
