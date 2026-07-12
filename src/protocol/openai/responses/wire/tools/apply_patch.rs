use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use strum::Display;

// ============================================================
// Input / Context Item Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ApplyPatchCallStatusParam {
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ApplyPatchCallOutputStatusParam {
    Completed,
    Failed,
}

/// OpenAPI schema: `#/components/schemas/ApplyPatchCreateFileOperationParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchCreateFileOperationParam {
    pub path: String,
    pub diff: String,
}

/// OpenAPI schema: `#/components/schemas/ApplyPatchDeleteFileOperationParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchDeleteFileOperationParam {
    pub path: String,
}

/// OpenAPI schema: `#/components/schemas/ApplyPatchUpdateFileOperationParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchUpdateFileOperationParam {
    pub path: String,
    pub diff: String,
}

#[allow(
    clippy::enum_variant_names,
    reason = "Mirrors OpenAI Responses apply-patch operation variant names."
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApplyPatchOperationParam {
    CreateFile(ApplyPatchCreateFileOperationParam),
    DeleteFile(ApplyPatchDeleteFileOperationParam),
    UpdateFile(ApplyPatchUpdateFileOperationParam),
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/ApplyPatchToolCallItemParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchToolCallItemParam {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub id: OptionalNullable<String>,
    pub call_id: String,
    pub status: ApplyPatchCallStatusParam,
    pub operation: ApplyPatchOperationParam,
}

/// OpenAPI schema: `#/components/schemas/ApplyPatchToolCallOutputItemParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchToolCallOutputItemParam {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub id: OptionalNullable<String>,
    pub call_id: String,
    pub status: ApplyPatchCallOutputStatusParam,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub output: OptionalNullable<String>,
}

// ============================================================
// Output / Resource Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ApplyPatchCallStatus {
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ApplyPatchCallOutputStatus {
    Completed,
    Failed,
}

/// OpenAPI schema: `#/components/schemas/ApplyPatchCreateFileOperation`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchCreateFileOperation {
    pub path: String,
    pub diff: String,
}

/// OpenAPI schema: `#/components/schemas/ApplyPatchDeleteFileOperation`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchDeleteFileOperation {
    pub path: String,
}

/// OpenAPI schema: `#/components/schemas/ApplyPatchUpdateFileOperation`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchUpdateFileOperation {
    pub path: String,
    pub diff: String,
}

#[allow(
    clippy::enum_variant_names,
    reason = "Mirrors OpenAI Responses apply-patch operation variant names."
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApplyPatchOperation {
    CreateFile(ApplyPatchCreateFileOperation),
    DeleteFile(ApplyPatchDeleteFileOperation),
    UpdateFile(ApplyPatchUpdateFileOperation),
}

// ============================================================
// Output / Resource Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/ApplyPatchToolCall`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchToolCall {
    pub id: String,
    pub call_id: String,
    pub status: ApplyPatchCallStatus,
    pub operation: ApplyPatchOperation,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub created_by: Option<String>,
}

/// OpenAPI schema: `#/components/schemas/ApplyPatchToolCallOutput`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchToolCallOutput {
    pub id: String,
    pub call_id: String,
    pub status: ApplyPatchCallOutputStatus,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub output: OptionalNullable<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub created_by: Option<String>,
}
