use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::Display;

use super::super::OutputStatus;
use super::Tool;
use super::function::{FunctionCallOutputStatusEnum, FunctionCallStatus};

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ToolSearchExecutionType {
    Server,
    Client,
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchToolParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ToolSearchExecutionType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchCallItemParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ToolSearchExecutionType>,
    #[serde(default)]
    pub arguments: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<OutputStatus>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSearchOutputItemParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ToolSearchExecutionType>,
    pub tools: Vec<Tool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<OutputStatus>,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchCall {
    pub id: String,
    pub call_id: Option<String>,
    pub execution: ToolSearchExecutionType,
    pub arguments: Value,
    pub status: FunctionCallStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSearchOutput {
    pub id: String,
    pub call_id: Option<String>,
    pub execution: ToolSearchExecutionType,
    pub tools: Vec<Tool>,
    pub status: FunctionCallOutputStatusEnum,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
}
