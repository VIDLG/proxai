use async_openai::types::responses as openai;
use serde_json::Value;
use structural_convert::StructuralConvert;
use strum::Display;

use super::super::OutputStatus;
use super::function::{FunctionCallOutputStatusEnum, FunctionCallStatus};
use super::Tool;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::ToolSearchExecutionType))]
#[strum(serialize_all = "snake_case")]
pub enum ToolSearchExecutionType {
    Server,
    Client,
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ToolSearchToolParam))]
pub struct ToolSearchToolParam {
    pub execution: Option<ToolSearchExecutionType>,
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ToolSearchCallItemParam))]
pub struct ToolSearchCallItemParam {
    pub id: Option<String>,
    pub call_id: Option<String>,
    pub execution: Option<ToolSearchExecutionType>,
    pub arguments: Value,
    pub status: Option<OutputStatus>,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::ToolSearchOutputItemParam))]
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

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ToolSearchCall))]
pub struct ToolSearchCall {
    pub id: String,
    pub call_id: Option<String>,
    pub execution: ToolSearchExecutionType,
    pub arguments: Value,
    pub status: FunctionCallStatus,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::ToolSearchOutput))]
pub struct ToolSearchOutput {
    pub id: String,
    pub call_id: Option<String>,
    pub execution: ToolSearchExecutionType,
    pub tools: Vec<Tool>,
    pub status: FunctionCallOutputStatusEnum,
    pub created_by: Option<String>,
}
