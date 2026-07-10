use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::Display;

// ============================================================
// Tool Choice
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceMCP {
    pub name: String,
    pub server_label: String,
}

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPToolFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MCPToolAllowedTools {
    List(Vec<String>),
    Filter(MCPToolFilter),
}

#[allow(
    clippy::enum_variant_names,
    reason = "Mirrors upstream MCP connector identifiers."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum MCPToolApprovalSetting {
    Always,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPToolApprovalFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always: Option<MCPToolFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub never: Option<MCPToolFilter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MCPToolRequireApproval {
    Filter(MCPToolApprovalFilter),
    ApprovalSetting(MCPToolApprovalSetting),
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPTool {
    pub server_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<MCPToolAllowedTools>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<McpToolConnectorId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_approval: Option<MCPToolRequireApproval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPApprovalResponse {
    pub approval_request_id: String,
    pub approve: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPApprovalRequest {
    pub arguments: String,
    pub id: String,
    pub name: String,
    pub server_label: String,
}

// ============================================================
// MCP List Tools Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPListToolsTool {
    pub input_schema: Value,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ============================================================
// MCP List Tools Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPListTools {
    pub id: String,
    pub server_label: String,
    pub tools: Vec<MCPListToolsTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================
// MCP Call Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
