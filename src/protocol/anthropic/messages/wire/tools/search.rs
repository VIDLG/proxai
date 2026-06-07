#![allow(
    dead_code,
    unused_imports,
    clippy::enum_variant_names,
    reason = "Anthropic Messages tool-search result schema mirrors upstream generated types."
)]

use serde::{Deserialize, Serialize};

use super::super::{
    blocks::TextBlockParam, citations::CitationsConfigParam, common::CacheControlEphemeral,
};

// ── Shared type discriminators ────────────────────────────────────────────

/// Discriminator value used by `ToolSearchToolResultError.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSearchToolResultErrorType {
    ToolSearchToolResultError,
}

/// Discriminator value used by `ToolSearchToolSearchResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSearchToolSearchResultType {
    ToolSearchToolSearchResult,
}

/// Discriminator value used by `ToolSearchToolResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSearchToolResultType {
    ToolSearchToolResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSearchToolResultErrorCode {
    InvalidToolInput,
    Unavailable,
    TooManyRequests,
    ExecutionTimeExceeded,
}

// ── ToolReference block param ──────────────────────────────────────────────

use super::tool_use::ToolReferenceBlockParam;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResultBlockParam {
    pub content: Vec<TextBlockParam>,
    pub source: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<CitationsConfigParam>,
}

// ═══════════════════════════════════════════════════════════════════════════
//  Response types (what the API returns)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchToolResultError {
    pub error_code: ToolSearchToolResultErrorCode,
    /// @sdk(required_nullable_accepts_missing)
    pub error_message: Option<String>,
    #[serde(rename = "type")]
    pub type_: ToolSearchToolResultErrorType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchToolSearchResultBlock {
    pub tool_references: Vec<ToolReferenceBlockParam>,
    #[serde(rename = "type")]
    pub type_: ToolSearchToolSearchResultType,
}

/// ToolSearchToolResultBlock.content: `ToolSearchToolResultError | ToolSearchToolSearchResultBlock`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolSearchToolResultBlockContent {
    Error(ToolSearchToolResultError),
    SearchResult(ToolSearchToolSearchResultBlock),
}

/// 🎯 @use: tool search tool result block — response-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchToolResultBlock {
    pub content: ToolSearchToolResultBlockContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: ToolSearchToolResultType,
}

// ═══════════════════════════════════════════════════════════════════════════
//  Request types (what you send to the API)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchToolResultErrorParam {
    pub error_code: ToolSearchToolResultErrorCode,
    #[serde(rename = "type")]
    pub type_: ToolSearchToolResultErrorType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchToolSearchResultBlockParam {
    pub tool_references: Vec<ToolReferenceBlockParam>,
    #[serde(rename = "type")]
    pub type_: ToolSearchToolSearchResultType,
}

/// ToolSearchToolResultBlockParam.content: `ToolSearchToolResultErrorParam | ToolSearchToolSearchResultBlockParam`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolSearchToolResultBlockParamContent {
    Error(ToolSearchToolResultErrorParam),
    SearchResult(ToolSearchToolSearchResultBlockParam),
}

/// 🎯 @use: tool search tool result block param — request-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchToolResultBlockParam {
    pub content: ToolSearchToolResultBlockParamContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: ToolSearchToolResultType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
}
