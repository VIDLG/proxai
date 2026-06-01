use async_openai::types::responses as openai;
use structural_convert::StructuralConvert;

use super::custom::CustomToolParam;
use super::function::FunctionToolParam;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::NamespaceToolParamTool))]
pub enum NamespaceToolParamTool {
    Function(FunctionToolParam),
    Custom(CustomToolParam),
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::NamespaceToolParam))]
pub struct NamespaceToolParam {
    pub name: String,
    pub description: String,
    pub tools: Vec<NamespaceToolParamTool>,
}
