use serde::{Deserialize, Serialize};

use super::custom::CustomToolParam;
use super::function::FunctionToolParam;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NamespaceToolParamTool {
    Function(FunctionToolParam),
    Custom(CustomToolParam),
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamespaceToolParam {
    pub name: String,
    pub description: String,
    pub tools: Vec<NamespaceToolParamTool>,
}
