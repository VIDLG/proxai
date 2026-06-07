use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::Display;

use super::function::{FunctionCallOutputStatusEnum, FunctionCallStatus};

// ============================================================
// Tool Choice
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceCustom {
    pub name: String,
}

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum GrammarSyntax {
    #[default]
    Lark,
    Regex,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CustomGrammarFormatParam {
    pub definition: String,
    pub syntax: GrammarSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CustomToolParamFormat {
    #[default]
    Text,
    Grammar(CustomGrammarFormatParam),
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[allow(dead_code, reason = "Retained for future item-resource modeling.")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomToolCallResource {
    pub call_id: String,
    pub namespace: Option<String>,
    pub input: String,
    pub name: String,
    pub id: String,
    pub status: FunctionCallStatus,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomToolCallOutputResource {
    pub call_id: String,
    pub output: CustomToolCallOutputOutput,
    pub id: String,
    pub status: FunctionCallOutputStatusEnum,
    pub created_by: Option<String>,
}
