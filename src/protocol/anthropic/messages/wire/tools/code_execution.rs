#![allow(
    dead_code,
    unused_imports,
    clippy::enum_variant_names,
    reason = "Anthropic Messages code-execution tool result schema mirrors upstream generated types."
)]

use serde::{Deserialize, Serialize};

use super::super::common::CacheControlEphemeral;

// ═══════════════════════════════════════════════════════════════════════════
//  Shared type discriminators
// ═══════════════════════════════════════════════════════════════════════════

/// Discriminator value used by `CodeExecutionOutputBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeExecutionOutputType {
    CodeExecutionOutput,
}

/// Discriminator value used by `CodeExecutionResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeExecutionResultType {
    CodeExecutionResult,
}

/// Discriminator value used by `EncryptedCodeExecutionResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncryptedCodeExecutionResultType {
    EncryptedCodeExecutionResult,
}

/// Discriminator value used by `CodeExecutionToolResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeExecutionToolResultType {
    CodeExecutionToolResult,
}

/// Discriminator value used by `CodeExecutionToolResultError.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeExecutionToolResultErrorType {
    CodeExecutionToolResultError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeExecutionToolResultErrorCode {
    InvalidToolInput,
    Unavailable,
    TooManyRequests,
    ExecutionTimeExceeded,
}

// ═══════════════════════════════════════════════════════════════════════════
//  Response types (what the API returns)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeExecutionOutputBlock {
    pub file_id: String,
    #[serde(rename = "type")]
    pub type_: CodeExecutionOutputType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeExecutionResultBlock {
    pub content: Vec<CodeExecutionOutputBlock>,
    pub return_code: i32,
    pub stderr: String,
    pub stdout: String,
    #[serde(rename = "type")]
    pub type_: CodeExecutionResultType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedCodeExecutionResultBlock {
    pub content: Vec<CodeExecutionOutputBlock>,
    pub encrypted_stdout: String,
    pub return_code: i32,
    pub stderr: String,
    #[serde(rename = "type")]
    pub type_: EncryptedCodeExecutionResultType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeExecutionToolResultError {
    pub error_code: CodeExecutionToolResultErrorCode,
    #[serde(rename = "type")]
    pub type_: CodeExecutionToolResultErrorType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CodeExecutionToolResultBlockContent {
    Error(CodeExecutionToolResultError),
    Result(CodeExecutionResultBlock),
    Encrypted(EncryptedCodeExecutionResultBlock),
}

/// 🎯 @use: code execution tool result block — response-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeExecutionToolResultBlock {
    pub content: CodeExecutionToolResultBlockContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: CodeExecutionToolResultType,
}

// ═══════════════════════════════════════════════════════════════════════════
//  Request types (what you send to the API)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeExecutionOutputBlockParam {
    pub file_id: String,
    #[serde(rename = "type")]
    pub type_: CodeExecutionOutputType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeExecutionResultBlockParam {
    pub content: Vec<CodeExecutionOutputBlockParam>,
    pub return_code: i32,
    pub stderr: String,
    pub stdout: String,
    #[serde(rename = "type")]
    pub type_: CodeExecutionResultType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedCodeExecutionResultBlockParam {
    pub content: Vec<CodeExecutionOutputBlockParam>,
    pub encrypted_stdout: String,
    pub return_code: i32,
    pub stderr: String,
    #[serde(rename = "type")]
    pub type_: EncryptedCodeExecutionResultType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeExecutionToolResultErrorParam {
    pub error_code: CodeExecutionToolResultErrorCode,
    #[serde(rename = "type")]
    pub type_: CodeExecutionToolResultErrorType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CodeExecutionToolResultBlockParamContent {
    Error(CodeExecutionToolResultErrorParam),
    Result(CodeExecutionResultBlockParam),
    Encrypted(EncryptedCodeExecutionResultBlockParam),
}

/// 🎯 @use: code execution tool result block param — request-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeExecutionToolResultBlockParam {
    pub content: CodeExecutionToolResultBlockParamContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: CodeExecutionToolResultType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
}
