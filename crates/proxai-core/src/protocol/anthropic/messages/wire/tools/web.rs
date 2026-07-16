#![allow(
    dead_code,
    reason = "Anthropic Messages web tool schema mirrors upstream generated types."
)]

use crate::protocol::RequiredNullable;
use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};

use super::super::{
    blocks::{DocumentBlock, DocumentBlockParam},
    common::CacheControlEphemeral,
};

use super::tool_use::ToolCaller;

// ── Shared type discriminators ────────────────────────────────────────────

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `WebFetchBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebFetchBlockType {
    WebFetchResult,
}

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `WebFetchToolResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebFetchToolResultType {
    WebFetchToolResult,
}

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `WebFetchToolResultErrorBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebFetchToolResultErrorType {
    WebFetchToolResultError,
}

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `WebSearchResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchResultType {
    WebSearchResult,
}

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `WebSearchToolResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchToolResultType {
    WebSearchToolResult,
}

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by web search tool-result error shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchToolResultErrorType {
    WebSearchToolResultError,
}

/// @sdk(shape = "WebFetchBlock")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebFetchBlock {
    pub content: DocumentBlock,
    pub retrieved_at: RequiredNullable<String>,
    #[serde(rename = "type")]
    pub type_: WebFetchBlockType,
    pub url: String,
}

/// @sdk(proxai_internal = "union_wrapper")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebFetchToolResultContent {
    Error(WebFetchToolResultErrorBlock),
    Block(WebFetchBlock),
}

// ── Web Fetch result block ─────────────────────────────────────────

/// @sdk(shape = "WebFetchToolResultBlock")
/// 🎯 @use: web fetch tool result block — response-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebFetchToolResultBlock {
    pub caller: ToolCaller,
    pub content: WebFetchToolResultContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: WebFetchToolResultType,
}

/// @sdk(shape = "WebFetchToolResultErrorCode")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebFetchToolResultErrorCode {
    InvalidToolInput,
    Unavailable,
    TooManyRequests,
    MaxUsesExceeded,
    UnsupportedContentType,
    UrlNotAccessible,
    UrlNotAllowed,
    UrlNotInPriorContext,
    UrlTooLong,
}

// ── Web Fetch error types ──────────────────────────────────────────

/// @sdk(shape = "WebFetchToolResultErrorBlock")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebFetchToolResultErrorBlock {
    pub error_code: WebFetchToolResultErrorCode,
    #[serde(rename = "type")]
    pub type_: WebFetchToolResultErrorType,
}

/// @sdk(shape = "WebSearchToolResultErrorCode")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchToolResultErrorCode {
    InvalidToolInput,
    Unavailable,
    MaxUsesExceeded,
    TooManyRequests,
    QueryTooLong,
    RequestTooLarge,
}

// ── Web Search result data types ────────────────────────────────────

/// @sdk(shape = "WebSearchResultBlock")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchResultBlock {
    pub encrypted_content: String,
    pub page_age: RequiredNullable<String>,
    pub title: String,
    #[serde(rename = "type")]
    pub type_: WebSearchResultType,
    pub url: String,
}

/// @sdk(shape = "WebSearchToolResultBlockContent")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebSearchToolResultBlockContent {
    Error(WebSearchToolResultError),
    Data(Vec<WebSearchResultBlock>),
}

// ── Web Search result block ────────────────────────────────────────

/// @sdk(shape = "WebSearchToolResultBlock")
/// 🎯 @use: web search tool result block — response-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchToolResultBlock {
    pub caller: ToolCaller,
    pub content: WebSearchToolResultBlockContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: WebSearchToolResultType,
}

// ── Web Search error types ─────────────────────────────────────────

/// @sdk(shape = "WebSearchToolResultError")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchToolResultError {
    pub error_code: WebSearchToolResultErrorCode,
    #[serde(rename = "type")]
    pub type_: WebSearchToolResultErrorType,
}

/// @sdk(shape = "WebSearchToolRequestError")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchToolRequestError {
    pub error_code: WebSearchToolResultErrorCode,
    #[serde(rename = "type")]
    pub type_: WebSearchToolResultErrorType,
}

// ═══════════════════════════════════════════════════════════════════════════
//  Request-side result types
// ═══════════════════════════════════════════════════════════════════════════

// ── Web Fetch param types ──────────────────────────────────────────

/// @sdk(shape = "WebFetchBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebFetchBlockParam {
    pub content: DocumentBlockParam,
    #[serde(rename = "type")]
    pub type_: WebFetchBlockType,
    pub url: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub retrieved_at: OptionalNullable<String>,
}

/// @sdk(proxai_internal = "union_wrapper")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebFetchToolResultParamContent {
    Error(WebFetchToolResultErrorBlockParam),
    Block(WebFetchBlockParam),
}

/// @sdk(shape = "WebFetchToolResultBlockParam")
/// 🎯 @use: web fetch tool result block param — request-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebFetchToolResultBlockParam {
    pub content: WebFetchToolResultParamContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: WebFetchToolResultType,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub caller: Option<ToolCaller>,
}

/// @sdk(shape = "WebFetchToolResultErrorBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebFetchToolResultErrorBlockParam {
    pub error_code: WebFetchToolResultErrorCode,
    #[serde(rename = "type")]
    pub type_: WebFetchToolResultErrorType,
}

// ── Web Search param types ─────────────────────────────────────────

/// @sdk(shape = "WebSearchResultBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchResultBlockParam {
    pub encrypted_content: String,
    pub title: String,
    #[serde(rename = "type")]
    pub type_: WebSearchResultType,
    pub url: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub page_age: OptionalNullable<String>,
}

/// @sdk(shape = "WebSearchToolResultBlockParamContent")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebSearchToolResultBlockParamContent {
    Error(WebSearchToolRequestError),
    Data(Vec<WebSearchResultBlockParam>),
}

/// @sdk(shape = "WebSearchToolResultBlockParam")
/// 🎯 @use: web search tool result block param — request-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchToolResultBlockParam {
    pub content: WebSearchToolResultBlockParamContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: WebSearchToolResultType,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub caller: Option<ToolCaller>,
}
