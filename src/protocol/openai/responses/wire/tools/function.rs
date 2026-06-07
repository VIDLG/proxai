use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::Display;

use super::super::{InputContent, OutputStatus};

// ============================================================
// Tool Choice
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceFunction {
    pub name: String,
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionTool {
    pub name: String,
    pub parameters: Option<Value>,
    pub strict: Option<bool>,
    pub description: Option<String>,
    pub defer_loading: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionToolParam {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
    pub strict: Option<bool>,
    pub defer_loading: Option<bool>,
}

// ============================================================
// Shared Function Status
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FunctionCallStatus {
    InProgress,
    Completed,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FunctionCallOutputStatusEnum {
    InProgress,
    Completed,
    Incomplete,
}

// ============================================================
// Shared Function Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionToolCall {
    pub arguments: String,
    pub call_id: String,
    pub namespace: Option<String>,
    pub name: String,
    pub id: Option<String>,
    pub status: Option<OutputStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionCallOutput {
    Text(String),
    Content(Vec<InputContent>),
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCallOutputItemParam {
    pub call_id: String,
    pub output: FunctionCallOutput,
    pub id: Option<String>,
    pub status: Option<OutputStatus>,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

#[allow(dead_code, reason = "Retained for future item-resource modeling.")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionToolCallResource {
    pub arguments: String,
    pub call_id: String,
    pub namespace: Option<String>,
    pub name: String,
    pub id: String,
    pub status: FunctionCallStatus,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionToolCallOutputResource {
    pub call_id: String,
    pub output: FunctionCallOutput,
    pub id: String,
    pub status: FunctionCallOutputStatusEnum,
    pub created_by: Option<String>,
}
