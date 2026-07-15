use crate::protocol::OptionalNullable;
use serde::{Deserialize, Serialize};
use strum::Display;

use std::collections::HashMap;

use super::super::OutputStatus;

// ============================================================
// Local Shell Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum LocalShellActionType {
    Exec,
}

/// OpenAPI schema: `#/components/schemas/LocalShellExecAction`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalShellExecAction {
    pub r#type: LocalShellActionType,
    pub command: Vec<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub timeout_ms: OptionalNullable<u64>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub working_directory: OptionalNullable<String>,
    pub env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub user: OptionalNullable<String>,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/LocalShellToolCallOutput`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalShellToolCallOutput {
    pub id: String,
    pub output: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub status: OptionalNullable<OutputStatus>,
}

// ============================================================
// Local Shell Output Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/LocalShellToolCall`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalShellToolCall {
    pub id: String,
    pub call_id: String,

    pub action: LocalShellExecAction,
    pub status: OutputStatus,
}
