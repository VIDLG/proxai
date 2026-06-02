use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use structural_convert::StructuralConvert;
use strum::Display;

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
#[serde(rename_all = "lowercase")]
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
#[serde(untagged)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CustomToolCallOutputOutput {
    Text(String),
    List(Vec<Value>),
}

impl From<openai::CustomToolCallOutputOutput> for CustomToolCallOutputOutput {
    fn from(value: openai::CustomToolCallOutputOutput) -> Self {
        serde_json::from_value(serde_json::to_value(value).unwrap_or_default())
            .expect("CustomToolCallOutputOutput should match local protocol shape")
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomToolCall {
    pub call_id: String,
    pub namespace: Option<String>,
    pub input: String,
    pub name: String,
    pub id: Option<String>,
}

impl From<openai::CustomToolCall> for CustomToolCall {
    fn from(value: openai::CustomToolCall) -> Self {
        Self {
            call_id: value.call_id,
            namespace: value.namespace,
            input: value.input,
            name: value.name,
            id: Some(value.id),
        }
    }
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
