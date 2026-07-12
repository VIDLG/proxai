use crate::protocol::deserialize_present;
use serde::{Deserialize, Serialize};
use strum::Display;

use super::super::InputContent;
use super::function::{FunctionCallOutputStatusEnum, FunctionCallStatus};

// ============================================================
// Tool Choice
// ============================================================

/// OpenAPI schema: `#/components/schemas/ToolChoiceCustom`
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

/// OpenAPI schema: `#/components/schemas/CustomGrammarFormatParam`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CustomGrammarFormatParam {
    pub syntax: GrammarSyntax,

    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CustomToolParamFormat {
    #[default]
    Text,
    Grammar(CustomGrammarFormatParam),
}

// ============================================================
// Tool Definition
// ============================================================

/// OpenAPI schema: `#/components/schemas/CustomToolParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomToolParam {
    pub name: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub description: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub format: Option<CustomToolParamFormat>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub defer_loading: Option<bool>,
}

// ============================================================
// Shared / Supporting Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CustomToolCallOutputOutput {
    Text(String),
    List(Vec<InputContent>),
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/CustomToolCallOutput`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomToolCallOutput {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub id: Option<String>,

    pub call_id: String,
    pub output: CustomToolCallOutputOutput,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/CustomToolCall`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomToolCall {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub id: Option<String>,

    pub call_id: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub namespace: Option<String>,
    pub name: String,
    pub input: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CustomToolCallType {
    CustomToolCall,
}

/// OpenAPI schema: `#/components/schemas/CustomToolCallResource`
#[allow(dead_code, reason = "Retained for future item-resource modeling.")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomToolCallResource {
    pub r#type: CustomToolCallType,
    pub id: String,
    pub call_id: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub namespace: Option<String>,
    pub name: String,
    pub input: String,
    pub status: FunctionCallStatus,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub created_by: Option<String>,
}

/// OpenAPI schema: `#/components/schemas/CustomToolCallOutputResource`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomToolCallOutputResource {
    pub id: String,

    pub call_id: String,
    pub output: CustomToolCallOutputOutput,
    pub status: FunctionCallOutputStatusEnum,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub created_by: Option<String>,
}
