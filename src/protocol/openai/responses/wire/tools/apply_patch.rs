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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchCreateFileOperationParam {
    pub path: String,
    pub diff: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchDeleteFileOperationParam {
    pub path: String,
}

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
pub enum ApplyPatchOperationParam {
    CreateFile(ApplyPatchCreateFileOperationParam),
    DeleteFile(ApplyPatchDeleteFileOperationParam),
    UpdateFile(ApplyPatchUpdateFileOperationParam),
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchToolCallItemParam {
    pub id: Option<String>,
    pub call_id: String,
    pub status: ApplyPatchCallStatusParam,
    pub operation: ApplyPatchOperationParam,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchToolCallOutputItemParam {
    pub id: Option<String>,
    pub call_id: String,
    pub status: ApplyPatchCallOutputStatusParam,
    pub output: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchCreateFileOperation {
    pub path: String,
    pub diff: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchDeleteFileOperation {
    pub path: String,
}

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
pub enum ApplyPatchOperation {
    CreateFile(ApplyPatchCreateFileOperation),
    DeleteFile(ApplyPatchDeleteFileOperation),
    UpdateFile(ApplyPatchUpdateFileOperation),
}

// ============================================================
// Output / Resource Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchToolCall {
    pub id: String,
    pub call_id: String,
    pub status: ApplyPatchCallStatus,
    pub operation: ApplyPatchOperation,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPatchToolCallOutput {
    pub id: String,
    pub call_id: String,
    pub status: ApplyPatchCallOutputStatus,
    pub output: Option<String>,
    pub created_by: Option<String>,
}
