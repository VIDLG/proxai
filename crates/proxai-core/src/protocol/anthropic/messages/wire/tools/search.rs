#![allow(
    dead_code,
    unused_imports,
    clippy::enum_variant_names,
    reason = "Anthropic Messages tool-search result schema mirrors upstream generated types."
)]

use crate::protocol::RequiredNullable;
use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};

use super::super::{
    blocks::TextBlockParam, citations::CitationsConfigParam, common::CacheControlEphemeral,
};

// ── Shared type discriminators ────────────────────────────────────────────

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `ToolSearchToolResultError.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSearchToolResultErrorType {
    ToolSearchToolResultError,
}

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `ToolSearchToolSearchResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSearchToolSearchResultType {
    ToolSearchToolSearchResult,
}

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `ToolSearchToolResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSearchToolResultType {
    ToolSearchToolResult,
}

/// @sdk(shape = "ToolSearchToolResultErrorCode")
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

/// @sdk(shape = "SearchResultBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResultBlockParam {
    pub content: Vec<TextBlockParam>,
    pub source: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub citations: Option<CitationsConfigParam>,
}

// ═══════════════════════════════════════════════════════════════════════════
//  Response types (what the API returns)
// ═══════════════════════════════════════════════════════════════════════════

/// @sdk(shape = "ToolSearchToolResultError")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchToolResultError {
    pub error_code: ToolSearchToolResultErrorCode,
    pub error_message: RequiredNullable<String>,
    #[serde(rename = "type")]
    pub type_: ToolSearchToolResultErrorType,
}

/// @sdk(shape = "ToolSearchToolSearchResultBlock")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchToolSearchResultBlock {
    pub tool_references: Vec<ToolReferenceBlockParam>,
    #[serde(rename = "type")]
    pub type_: ToolSearchToolSearchResultType,
}

/// @sdk(proxai_internal = "union_wrapper")
/// ToolSearchToolResultBlock.content: `ToolSearchToolResultError | ToolSearchToolSearchResultBlock`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolSearchToolResultBlockContent {
    Error(ToolSearchToolResultError),
    SearchResult(ToolSearchToolSearchResultBlock),
}

/// @sdk(shape = "ToolSearchToolResultBlock")
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

/// @sdk(shape = "ToolSearchToolResultErrorParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchToolResultErrorParam {
    pub error_code: ToolSearchToolResultErrorCode,
    #[serde(rename = "type")]
    pub type_: ToolSearchToolResultErrorType,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub error_message: OptionalNullable<String>,
}

/// @sdk(shape = "ToolSearchToolSearchResultBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchToolSearchResultBlockParam {
    pub tool_references: Vec<ToolReferenceBlockParam>,
    #[serde(rename = "type")]
    pub type_: ToolSearchToolSearchResultType,
}

/// @sdk(proxai_internal = "union_wrapper")
/// ToolSearchToolResultBlockParam.content: `ToolSearchToolResultErrorParam | ToolSearchToolSearchResultBlockParam`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolSearchToolResultBlockParamContent {
    Error(ToolSearchToolResultErrorParam),
    SearchResult(ToolSearchToolSearchResultBlockParam),
}

/// @sdk(shape = "ToolSearchToolResultBlockParam")
/// 🎯 @use: tool search tool result block param — request-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSearchToolResultBlockParam {
    pub content: ToolSearchToolResultBlockParamContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: ToolSearchToolResultType,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
}
