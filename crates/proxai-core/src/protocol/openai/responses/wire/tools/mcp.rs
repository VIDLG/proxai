use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::Display;

use super::CallableToolAllowedCaller;

// ============================================================
// Tool Choice
// ============================================================

/// OpenAPI schema: `#/components/schemas/ToolChoiceMCP`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceMCP {
    pub server_label: String,

    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub name: OptionalNullable<String>,
}

// ============================================================
// Tool Definition Supporting Types
// ============================================================

/// OpenAPI schema: `#/components/schemas/MCPToolFilter`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPToolFilter {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub tool_names: Option<Vec<String>>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub read_only: Option<bool>,
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

/// OpenAPI schema: `#/components/schemas/MCPTool/properties/require_approval/anyOf/0/oneOf/0`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPToolApprovalFilter {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub always: Option<MCPToolFilter>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
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

/// OpenAPI schema: `#/components/schemas/MCPTool`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPTool {
    pub server_label: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub server_url: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub connector_id: Option<McpToolConnectorId>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub tunnel_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub authorization: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub server_description: Option<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub headers: OptionalNullable<Value>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub allowed_tools: OptionalNullable<MCPToolAllowedTools>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub allowed_callers: OptionalNullable<Vec<CallableToolAllowedCaller>>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub require_approval: OptionalNullable<MCPToolRequireApproval>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub defer_loading: Option<bool>,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/MCPApprovalResponse`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPApprovalResponse {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub id: OptionalNullable<String>,

    pub approval_request_id: String,
    pub approve: bool,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub reason: OptionalNullable<String>,
}

/// OpenAPI schema: `#/components/schemas/MCPApprovalResponseResource`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPApprovalResponseResource {
    pub id: String,
    pub approval_request_id: String,
    pub approve: bool,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub reason: OptionalNullable<String>,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/MCPApprovalRequest`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPApprovalRequest {
    pub id: String,
    pub server_label: String,
    pub name: String,

    pub arguments: String,
}

// ============================================================
// MCP List Tools Supporting Types
// ============================================================

/// OpenAPI schema: `#/components/schemas/MCPListToolsTool`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPListToolsTool {
    pub name: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub description: OptionalNullable<String>,

    pub input_schema: Value,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub annotations: OptionalNullable<Value>,
}

// ============================================================
// MCP List Tools Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/MCPListTools`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPListTools {
    pub id: String,
    pub server_label: String,
    pub tools: Vec<MCPListToolsTool>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub error: OptionalNullable<String>,
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

/// OpenAPI schema: `#/components/schemas/MCPToolCall`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MCPToolCall {
    pub id: String,
    pub server_label: String,
    pub name: String,

    pub arguments: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub output: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub error: OptionalNullable<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub status: Option<MCPToolCallStatus>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub approval_request_id: OptionalNullable<String>,
}
