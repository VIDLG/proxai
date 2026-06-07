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
    pub execution: Option<ToolSearchExecutionType>,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchCallItemParam {
    pub id: Option<String>,
    pub call_id: Option<String>,
    pub execution: Option<ToolSearchExecutionType>,
    pub arguments: Value,
    pub status: Option<OutputStatus>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSearchOutputItemParam {
    pub id: Option<String>,
    pub call_id: Option<String>,
    pub execution: Option<ToolSearchExecutionType>,
    pub tools: Vec<Tool>,
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
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSearchOutput {
    pub id: String,
    pub call_id: Option<String>,
    pub execution: ToolSearchExecutionType,
    pub tools: Vec<Tool>,
    pub status: FunctionCallOutputStatusEnum,
    pub created_by: Option<String>,
}
