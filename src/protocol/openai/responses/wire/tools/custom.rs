use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;
use strum::Display;

use super::super::InputContent;
use super::function::{FunctionCallOutputStatusEnum, FunctionCallStatus};

// ============================================================
// Tool Choice
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ToolChoiceCustom))]
pub struct ToolChoiceCustom {
    pub name: String,
}

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default, Serialize, Deserialize,
)]
#[convert(from(openai::GrammarSyntax))]
#[strum(serialize_all = "lowercase")]
pub enum GrammarSyntax {
    #[default]
    Lark,
    Regex,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Default, Serialize, Deserialize)]
#[convert(from(openai::CustomGrammarFormatParam))]
pub struct CustomGrammarFormatParam {
    pub definition: String,
    pub syntax: GrammarSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Default, Serialize, Deserialize)]
#[convert(from(openai::CustomToolParamFormat))]
pub enum CustomToolParamFormat {
    #[default]
    Text,
    Grammar(CustomGrammarFormatParam),
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CustomToolParam))]
pub struct CustomToolParam {
    pub name: String,
    pub description: Option<String>,
    pub format: CustomToolParamFormat,
    pub defer_loading: Option<bool>,
}

// ============================================================
// Shared / Supporting Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CustomToolCallOutputOutput))]
pub enum CustomToolCallOutputOutput {
    Text(String),
    List(Vec<InputContent>),
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CustomToolCallOutput))]
pub struct CustomToolCallOutput {
    pub call_id: String,
    pub output: CustomToolCallOutputOutput,
    pub id: Option<String>,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CustomToolCall))]
pub struct CustomToolCall {
    pub call_id: String,
    pub namespace: Option<String>,
    pub input: String,
    pub name: String,
    pub id: String,
}

#[allow(dead_code, reason = "Retained for future item-resource modeling.")]
#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CustomToolCallResource))]
pub struct CustomToolCallResource {
    pub call_id: String,
    pub namespace: Option<String>,
    pub input: String,
    pub name: String,
    pub id: String,
    pub status: FunctionCallStatus,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CustomToolCallOutputResource))]
pub struct CustomToolCallOutputResource {
    pub call_id: String,
    pub output: CustomToolCallOutputOutput,
    pub id: String,
    pub status: FunctionCallOutputStatusEnum,
    pub created_by: Option<String>,
}
