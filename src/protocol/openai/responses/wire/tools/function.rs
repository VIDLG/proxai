use crate::protocol::RequiredNullable;
use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::Display;

use super::super::{InputContent, OutputStatus};

// ============================================================
// Tool Choice
// ============================================================

/// OpenAPI schema: `#/components/schemas/ToolChoiceFunction`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceFunction {
    pub name: String,
}

// ============================================================
// Tool Definition
// ============================================================

/// OpenAPI schema: `#/components/schemas/FunctionTool`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionTool {
    pub name: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub description: OptionalNullable<String>,
    pub parameters: RequiredNullable<Value>,
    pub strict: RequiredNullable<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub defer_loading: Option<bool>,
}

/// OpenAPI schema: `#/components/schemas/FunctionToolParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionToolParam {
    pub name: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub description: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub parameters: OptionalNullable<Value>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub strict: OptionalNullable<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
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

/// OpenAPI schema: `#/components/schemas/FunctionToolCall`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionToolCall {
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

    pub arguments: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
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

/// OpenAPI schema: `#/components/schemas/FunctionCallOutputItemParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCallOutputItemParam {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub id: OptionalNullable<String>,

    pub call_id: String,
    pub output: FunctionCallOutput,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub status: OptionalNullable<OutputStatus>,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FunctionToolCallType {
    FunctionCall,
}

/// OpenAPI schema: `#/components/schemas/FunctionToolCallResource`
#[allow(dead_code, reason = "Retained for future item-resource modeling.")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionToolCallResource {
    pub id: String,

    pub r#type: FunctionToolCallType,
    pub call_id: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub namespace: Option<String>,
    pub name: String,
    pub arguments: String,
    pub status: FunctionCallStatus,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub created_by: Option<String>,
}

/// OpenAPI schema: `#/components/schemas/FunctionToolCallOutputResource`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionToolCallOutputResource {
    pub id: String,

    pub call_id: String,
    pub output: FunctionCallOutput,
    pub status: FunctionCallOutputStatusEnum,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub created_by: Option<String>,
}
