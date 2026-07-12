use serde::{Deserialize, Serialize};

use super::custom::CustomToolParam;
use super::function::FunctionToolParam;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NamespaceToolParamTool {
    Function(FunctionToolParam),
    Custom(CustomToolParam),
}

// ============================================================
// Tool Definition
// ============================================================

/// OpenAPI schema: `#/components/schemas/NamespaceToolParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamespaceToolParam {
    pub name: String,
    pub description: String,
    pub tools: Vec<NamespaceToolParamTool>,
}
