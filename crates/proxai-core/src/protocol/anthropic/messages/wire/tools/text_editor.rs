#![allow(
    dead_code,
    unused_imports,
    clippy::enum_variant_names,
    reason = "Anthropic Messages text-editor tool result schema mirrors upstream generated types."
)]

use crate::protocol::OptionalNullable;
use crate::protocol::RequiredNullable;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::common::CacheControlEphemeral;

// ═══════════════════════════════════════════════════════════════════════════
//  Shared type discriminators
// ═══════════════════════════════════════════════════════════════════════════

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `TextEditorCodeExecutionCreateResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionCreateResultType {
    TextEditorCodeExecutionCreateResult,
}

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `TextEditorCodeExecutionViewResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionViewResultType {
    TextEditorCodeExecutionViewResult,
}

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `TextEditorCodeExecutionStrReplaceResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionStrReplaceResultType {
    TextEditorCodeExecutionStrReplaceResult,
}

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `TextEditorCodeExecutionToolResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionToolResultType {
    TextEditorCodeExecutionToolResult,
}

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `TextEditorCodeExecutionToolResultError.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionToolResultErrorType {
    TextEditorCodeExecutionToolResultError,
}

/// @sdk(proxai_internal = "discriminator")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextEditorFileType {
    Text,
    Image,
    Pdf,
}

/// @sdk(shape = "TextEditorCodeExecutionToolResultErrorCode")
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

/// @sdk(shape = "TextEditorCodeExecutionCreateResultBlock")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionCreateResultBlock {
    pub is_file_update: bool,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionCreateResultType,
}

/// @sdk(shape = "TextEditorCodeExecutionStrReplaceResultBlock")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionStrReplaceResultBlock {
    pub lines: RequiredNullable<Vec<String>>,
    pub new_lines: RequiredNullable<u32>,
    pub new_start: RequiredNullable<u32>,
    pub old_lines: RequiredNullable<u32>,
    pub old_start: RequiredNullable<u32>,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionStrReplaceResultType,
}

/// @sdk(shape = "TextEditorCodeExecutionViewResultBlock")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionViewResultBlock {
    pub content: String,
    pub file_type: TextEditorFileType,
    pub num_lines: RequiredNullable<u32>,
    pub start_line: RequiredNullable<u32>,
    pub total_lines: RequiredNullable<u32>,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionViewResultType,
}

/// @sdk(shape = "TextEditorCodeExecutionToolResultError")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionToolResultError {
    pub error_code: TextEditorCodeExecutionToolResultErrorCode,
    pub error_message: RequiredNullable<String>,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionToolResultErrorType,
}

/// @sdk(proxai_internal = "union_wrapper")
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

/// @sdk(shape = "TextEditorCodeExecutionToolResultBlock")
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

/// @sdk(shape = "TextEditorCodeExecutionCreateResultBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionCreateResultBlockParam {
    pub is_file_update: bool,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionCreateResultType,
}

/// @sdk(shape = "TextEditorCodeExecutionStrReplaceResultBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionStrReplaceResultBlockParam {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub lines: OptionalNullable<Vec<String>>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub new_lines: OptionalNullable<u32>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub new_start: OptionalNullable<u32>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub old_lines: OptionalNullable<u32>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub old_start: OptionalNullable<u32>,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionStrReplaceResultType,
}

/// @sdk(shape = "TextEditorCodeExecutionViewResultBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionViewResultBlockParam {
    pub content: String,
    pub file_type: TextEditorFileType,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub num_lines: OptionalNullable<u32>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub start_line: OptionalNullable<u32>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub total_lines: OptionalNullable<u32>,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionViewResultType,
}

/// @sdk(shape = "TextEditorCodeExecutionToolResultErrorParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionToolResultErrorParam {
    pub error_code: TextEditorCodeExecutionToolResultErrorCode,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub error_message: OptionalNullable<String>,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionToolResultErrorType,
}

/// @sdk(proxai_internal = "union_wrapper")
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

/// @sdk(shape = "TextEditorCodeExecutionToolResultBlockParam")
/// 🎯 @use: text editor tool result block param — request-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEditorCodeExecutionToolResultBlockParam {
    pub content: TextEditorCodeExecutionToolResultParamContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: TextEditorCodeExecutionToolResultType,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
}
