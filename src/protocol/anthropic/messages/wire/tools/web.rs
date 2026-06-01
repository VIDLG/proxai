#![allow(
    dead_code,
    reason = "Anthropic Messages web tool schema mirrors upstream generated types."
)]

use serde::{Deserialize, Serialize};

use super::super::{
    blocks::{DocumentBlock, DocumentBlockParam},
    common::CacheControlEphemeral,
};

use super::tool_use::ToolCaller;

// ── Shared type discriminators ────────────────────────────────────────────

/// Discriminator value used by `WebFetchBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebFetchBlockType {
    WebFetchResult,
}

/// Discriminator value used by `WebFetchToolResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebFetchToolResultType {
    WebFetchToolResult,
}

/// Discriminator value used by `WebFetchToolResultErrorBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebFetchToolResultErrorType {
    WebFetchToolResultError,
}

/// Discriminator value used by `WebSearchResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchResultType {
    WebSearchResult,
}

/// Discriminator value used by `WebSearchToolResultBlock.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchToolResultType {
    WebSearchToolResult,
}

/// Discriminator value used by web search tool-result error shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchToolResultErrorType {
    WebSearchToolResultError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebFetchBlock {
    pub content: DocumentBlock,
    /// @sdk(required_nullable_accepts_missing)
    pub retrieved_at: Option<String>,
    #[serde(rename = "type")]
    pub type_: WebFetchBlockType,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebFetchToolResultContent {
    Data(Vec<WebFetchBlock>),
}

// ── Web Fetch result block ─────────────────────────────────────────

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
    UrlTooLong,
}

// ── Web Fetch error types ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebFetchToolResultErrorBlock {
    pub error_code: WebFetchToolResultErrorCode,
    #[serde(rename = "type")]
    pub type_: WebFetchToolResultErrorType,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchResultBlock {
    pub encrypted_content: String,
    /// @sdk(required_nullable_accepts_missing)
    pub page_age: Option<String>,
    pub title: String,
    #[serde(rename = "type")]
    pub type_: WebSearchResultType,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebSearchToolResultBlockContent {
    Error(WebSearchToolResultError),
    Data(Vec<WebSearchResultBlock>),
}

// ── Web Search result block ────────────────────────────────────────

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchToolResultError {
    pub error_code: WebSearchToolResultErrorCode,
    #[serde(rename = "type")]
    pub type_: WebSearchToolResultErrorType,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebFetchBlockParam {
    pub content: DocumentBlockParam,
    #[serde(rename = "type")]
    pub type_: WebFetchBlockType,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieved_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebFetchToolResultParamContent {
    Data(Vec<WebFetchBlockParam>),
}

/// 🎯 @use: web fetch tool result block param — request-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebFetchToolResultBlockParam {
    pub content: WebFetchToolResultParamContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: WebFetchToolResultType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller: Option<ToolCaller>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebFetchToolResultErrorBlockParam {
    pub error_code: WebFetchToolResultErrorCode,
    #[serde(rename = "type")]
    pub type_: WebFetchToolResultErrorType,
}

// ── Web Search param types ─────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchResultBlockParam {
    pub encrypted_content: String,
    pub title: String,
    #[serde(rename = "type")]
    pub type_: WebSearchResultType,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_age: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebSearchToolResultBlockParamContent {
    Error(WebSearchToolRequestError),
    Data(Vec<WebSearchResultBlockParam>),
}

/// 🎯 @use: web search tool result block param — request-side content block.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchToolResultBlockParam {
    pub content: WebSearchToolResultBlockParamContent,
    pub tool_use_id: String,
    #[serde(rename = "type")]
    pub type_: WebSearchToolResultType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller: Option<ToolCaller>,
}
