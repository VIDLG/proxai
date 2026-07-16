use super::{ContainerMemoryLimit, ContainerNetworkPolicy};
use crate::protocol::RequiredNullable;
use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use strum::Display;

use super::CallableToolAllowedCaller;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

/// OpenAPI schema: `#/components/schemas/AutoCodeInterpreterToolParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoCodeInterpreterToolParam {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub file_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub memory_limit: OptionalNullable<ContainerMemoryLimit>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub network_policy: Option<ContainerNetworkPolicy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CodeInterpreterToolContainer {
    Auto(AutoCodeInterpreterToolParam),
    #[serde(untagged)]
    ContainerID(String),
}

// ============================================================
// Tool Definition
// ============================================================

/// OpenAPI schema: `#/components/schemas/CodeInterpreterTool`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeInterpreterTool {
    pub container: CodeInterpreterToolContainer,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub allowed_callers: OptionalNullable<Vec<CallableToolAllowedCaller>>,
}

// ============================================================
// Output / Resource Supporting Types
// ============================================================

/// OpenAPI schema: `#/components/schemas/CodeInterpreterOutputLogs`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeInterpreterOutputLogs {
    pub logs: String,
}

/// OpenAPI schema: `#/components/schemas/CodeInterpreterOutputImage`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeInterpreterOutputImage {
    pub url: String,
}

// The upstream SDK also exposes `CodeInterpreterFile`, but the current
// `CodeInterpreterToolCallOutput` response shape here only exposes `Logs` and
// `Image` variants, so we do not model a separate local file output type yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CodeInterpreterToolCallOutput {
    Logs(CodeInterpreterOutputLogs),
    Image(CodeInterpreterOutputImage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CodeInterpreterToolCallStatus {
    InProgress,
    Completed,
    Incomplete,
    Interpreting,
    Failed,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/CodeInterpreterToolCall`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeInterpreterToolCall {
    pub id: String,
    pub status: CodeInterpreterToolCallStatus,
    pub container_id: String,

    pub code: RequiredNullable<String>,
    pub outputs: RequiredNullable<Vec<CodeInterpreterToolCallOutput>>,
}
