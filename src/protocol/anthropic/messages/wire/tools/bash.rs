#![allow(
    dead_code,
    unused_imports,
    clippy::enum_variant_names,
    reason = "Anthropic Messages bash tool result schema mirrors upstream generated types."
)]

use serde::{Deserialize, Serialize};

use super::super::common::CacheControlEphemeral;

// ═══════════════════════════════════════════════════════════════════════════
//  Shared type discriminators
// ═══════════════════════════════════════════════════════════════════════════

/// Discriminator value used by `BashCodeExecutionOutputBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BashCodeExecutionOutputType {
    BashCodeExecutionOutput,
}

/// Discriminator value used by `BashCodeExecutionResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BashCodeExecutionResultType {
    BashCodeExecutionResult,
}

/// Discriminator value used by `BashCodeExecutionToolResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BashCodeExecutionToolResultType {
    BashCodeExecutionToolResult,
}

/// Discriminator value used by `BashCodeExecutionToolResultError.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BashCodeExecutionToolResultErrorType {
    BashCodeExecutionToolResultError,
}

// ═══════════════════════════════════════════════════════════════════════════
//  Response types (what the API returns)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BashCodeExecutionOutputBlock {
    pub file_id: String,
    #[serde(rename = "type")]
    pub type_: BashCodeExecutionOutputType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BashCodeExecutionResultBlock {
    pub content: Vec<BashCodeExecutionOutputBlock>,
    pub return_code: i32,
    pub stderr: String,
    pub stdout: String,
    #[serde(rename = "type")]
    pub type_: BashCodeExecutionResultType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BashCodeExecutionToolResultErrorCode {
    InvalidToolInput,
    Unavailable,
    TooManyRequests,
    ExecutionTimeExceeded,
    OutputFileTooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BashCodeExecutionToolResultError {
    pub error_code: BashCodeExecutionToolResultErrorCode,
    #[serde(rename = "type")]
    pub type_: BashCodeExecutionToolResultErrorType,
}

/// BashCodeExecutionToolResultBlock.content: `BashCodeExecutionToolResultError | BashCodeExecutionResultBlock`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BashCodeExecutionToolResultContent {
    Error(BashCodeExecutionToolResultError),
    Result(BashCodeExecutionResultBlock),
}

/// 🎯 @use: bash code execution tool result block — response-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BashCodeExecutionToolResultBlock {
    pub content: BashCodeExecutionToolResultContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: BashCodeExecutionToolResultType,
}

// ═══════════════════════════════════════════════════════════════════════════
//  Request types (what you send to the API)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BashCodeExecutionOutputBlockParam {
    pub file_id: String,
    #[serde(rename = "type")]
    pub type_: BashCodeExecutionOutputType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BashCodeExecutionResultBlockParam {
    pub content: Vec<BashCodeExecutionOutputBlockParam>,
    pub return_code: i32,
    pub stderr: String,
    pub stdout: String,
    #[serde(rename = "type")]
    pub type_: BashCodeExecutionResultType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BashCodeExecutionToolResultErrorParam {
    pub error_code: BashCodeExecutionToolResultErrorCode,
    #[serde(rename = "type")]
    pub type_: BashCodeExecutionToolResultErrorType,
}

/// BashCodeExecutionToolResultBlockParam.content: `BashCodeExecutionToolResultErrorParam | BashCodeExecutionResultBlockParam`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BashCodeExecutionToolResultParamContent {
    Error(BashCodeExecutionToolResultErrorParam),
    Result(BashCodeExecutionResultBlockParam),
}

/// 🎯 @use: bash code execution tool result block param — request-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BashCodeExecutionToolResultBlockParam {
    pub content: BashCodeExecutionToolResultParamContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: BashCodeExecutionToolResultType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
}
