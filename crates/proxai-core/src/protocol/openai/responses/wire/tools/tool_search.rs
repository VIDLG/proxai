use crate::protocol::RequiredNullable;
use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::Display;

use super::super::OutputStatus;
use super::Tool;
use super::function::{FunctionCallOutputStatusEnum, FunctionCallStatus};

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ToolSearchExecutionType {
    Server,
    Client,
}

// ============================================================
// Tool Definition
// ============================================================

/// OpenAPI schema: `#/components/schemas/ToolSearchToolParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchToolParam {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub execution: Option<ToolSearchExecutionType>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub description: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub parameters: OptionalNullable<Value>,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/ToolSearchCallItemParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchCallItemParam {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub id: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub call_id: OptionalNullable<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub execution: Option<ToolSearchExecutionType>,
    #[serde(default)]
    pub arguments: Value,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub status: OptionalNullable<OutputStatus>,
}

/// OpenAPI schema: `#/components/schemas/ToolSearchOutputItemParam`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSearchOutputItemParam {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub id: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub call_id: OptionalNullable<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub execution: Option<ToolSearchExecutionType>,
    pub tools: Vec<Tool>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub status: OptionalNullable<OutputStatus>,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/ToolSearchCall`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchCall {
    pub id: String,
    pub call_id: RequiredNullable<String>,
    pub execution: ToolSearchExecutionType,
    pub arguments: Value,
    pub status: FunctionCallStatus,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub created_by: Option<String>,
}

/// OpenAPI schema: `#/components/schemas/ToolSearchOutput`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSearchOutput {
    pub id: String,
    pub call_id: RequiredNullable<String>,
    pub execution: ToolSearchExecutionType,
    pub tools: Vec<Tool>,
    pub status: FunctionCallOutputStatusEnum,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub created_by: Option<String>,
}
