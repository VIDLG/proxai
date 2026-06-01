#![allow(
    dead_code,
    unused_imports,
    clippy::enum_variant_names,
    reason = "Anthropic Messages text-editor tool result schema mirrors upstream generated types."
)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::common::CacheControlEphemeral;

// ═══════════════════════════════════════════════════════════════════════════
//  Shared type discriminators
// ═══════════════════════════════════════════════════════════════════════════

/// Discriminator value used by `TextEditorCodeExecutionCreateResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionCreateResultType {
    TextEditorCodeExecutionCreateResult,
}

/// Discriminator value used by `TextEditorCodeExecutionViewResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionViewResultType {
    TextEditorCodeExecutionViewResult,
}

/// Discriminator value used by `TextEditorCodeExecutionStrReplaceResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionStrReplaceResultType {
    TextEditorCodeExecutionStrReplaceResult,
}

/// Discriminator value used by `TextEditorCodeExecutionToolResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionToolResultType {
    TextEditorCodeExecutionToolResult,
}

/// Discriminator value used by `TextEditorCodeExecutionToolResultError.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionToolResultErrorType {
    TextEditorCodeExecutionToolResultError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextEditorFileType {
    Text,
    Image,
    Pdf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionToolResultErrorCode {
    InvalidToolInput,
    Unavailable,
    TooManyRequests,
    ExecutionTimeExceeded,
    FileNotFound,
}

// ═══════════════════════════════════════════════════════════════════════════
//  Response types (what the API returns)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionCreateResultBlock {
    pub is_file_update: bool,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionCreateResultType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionStrReplaceResultBlock {
    /// @sdk(required_nullable_accepts_missing)
    pub lines: Option<Vec<String>>,
    /// @sdk(required_nullable_accepts_missing)
    pub new_lines: Option<u32>,
    /// @sdk(required_nullable_accepts_missing)
    pub new_start: Option<u32>,
    /// @sdk(required_nullable_accepts_missing)
    pub old_lines: Option<u32>,
    /// @sdk(required_nullable_accepts_missing)
    pub old_start: Option<u32>,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionStrReplaceResultType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionViewResultBlock {
    pub content: String,
    pub file_type: TextEditorFileType,
    /// @sdk(required_nullable_accepts_missing)
    pub num_lines: Option<u32>,
    /// @sdk(required_nullable_accepts_missing)
    pub start_line: Option<u32>,
    /// @sdk(required_nullable_accepts_missing)
    pub total_lines: Option<u32>,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionViewResultType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionToolResultError {
    pub error_code: TextEditorCodeExecutionToolResultErrorCode,
    /// @sdk(required_nullable_accepts_missing)
    pub error_message: Option<String>,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionToolResultErrorType,
}

/// TextEditorCodeExecutionToolResultBlock.content:
///   `TextEditorCodeExecutionToolResultError | TextEditorCodeExecutionViewResultBlock | TextEditorCodeExecutionCreateResultBlock | TextEditorCodeExecutionStrReplaceResultBlock`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextEditorCodeExecutionToolResultContent {
    Error(TextEditorCodeExecutionToolResultError),
    View(TextEditorCodeExecutionViewResultBlock),
    Create(TextEditorCodeExecutionCreateResultBlock),
    StrReplace(TextEditorCodeExecutionStrReplaceResultBlock),
}

/// 🎯 @use: text editor tool result block — response-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionToolResultBlock {
    pub content: TextEditorCodeExecutionToolResultContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionToolResultType,
}

// ═══════════════════════════════════════════════════════════════════════════
//  Request types (what you send to the API)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionCreateResultBlockParam {
    pub is_file_update: bool,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionCreateResultType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionStrReplaceResultBlockParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_lines: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_start: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_lines: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_start: Option<u32>,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionStrReplaceResultType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionViewResultBlockParam {
    pub content: String,
    pub file_type: TextEditorFileType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_lines: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_lines: Option<u32>,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionViewResultType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionToolResultErrorParam {
    pub error_code: TextEditorCodeExecutionToolResultErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionToolResultErrorType,
}

/// TextEditorCodeExecutionToolResultBlockParam.content:
///   `TextEditorCodeExecutionToolResultErrorParam | TextEditorCodeExecutionViewResultBlockParam | TextEditorCodeExecutionCreateResultBlockParam | TextEditorCodeExecutionStrReplaceResultBlockParam`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextEditorCodeExecutionToolResultParamContent {
    Error(TextEditorCodeExecutionToolResultErrorParam),
    View(TextEditorCodeExecutionViewResultBlockParam),
    Create(TextEditorCodeExecutionCreateResultBlockParam),
    StrReplace(TextEditorCodeExecutionStrReplaceResultBlockParam),
}

/// 🎯 @use: text editor tool result block param — request-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionToolResultBlockParam {
    pub content: TextEditorCodeExecutionToolResultParamContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionToolResultType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
}
