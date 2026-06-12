#![allow(
    dead_code,
    unused_imports,
    clippy::enum_variant_names,
    reason = "Anthropic Messages citation schema mirrors upstream generated types."
)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Request types ─────────────────────────────────────────────────────────────

// ── Request config types ──

/// 🎯 @use: request-side citation toggle — enables/disables citation support.
/// Used by: blocks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationsConfig {
    pub enabled: bool,
}

/// 🎯 @use: request-side optional citation toggle.
/// Used by: blocks, search, tools
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationsConfigParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

// ── Request location types ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationCharLocationParam {
    pub cited_text: String,
    pub document_index: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub document_title: Option<String>,
    pub end_char_index: u32,
    pub start_char_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationPageLocationParam {
    pub cited_text: String,
    pub document_index: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub document_title: Option<String>,
    pub end_page_number: u32,
    pub start_page_number: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationContentBlockLocationParam {
    pub cited_text: String,
    pub document_index: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub document_title: Option<String>,
    pub end_block_index: u32,
    pub start_block_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationWebSearchResultLocationParam {
    pub cited_text: String,
    pub encrypted_index: String,
    /// @sdk(required_nullable_accepts_missing)
    pub title: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationSearchResultLocationParam {
    pub cited_text: String,
    pub end_block_index: u32,
    pub search_result_index: u32,
    pub source: String,
    pub start_block_index: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub title: Option<String>,
}

/// 🎯 @use: request-side citation variant — exactly one location type is active.
/// Used by: blocks, request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TextCitationParam {
    CharLocation(CitationCharLocationParam),
    PageLocation(CitationPageLocationParam),
    ContentBlockLocation(CitationContentBlockLocationParam),
    WebSearchResultLocation(CitationWebSearchResultLocationParam),
    SearchResultLocation(CitationSearchResultLocationParam),
}

// ── Response types ────────────────────────────────────────────────────────────

// ── Response location types ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationCharLocation {
    pub cited_text: String,
    pub document_index: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub document_title: Option<String>,
    pub end_char_index: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub file_id: Option<String>,
    pub start_char_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationPageLocation {
    pub cited_text: String,
    pub document_index: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub document_title: Option<String>,
    pub end_page_number: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub file_id: Option<String>,
    pub start_page_number: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationContentBlockLocation {
    pub cited_text: String,
    pub document_index: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub document_title: Option<String>,
    pub end_block_index: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub file_id: Option<String>,
    pub start_block_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationsWebSearchResultLocation {
    pub cited_text: String,
    pub encrypted_index: String,
    /// @sdk(required_nullable_accepts_missing)
    pub title: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationsSearchResultLocation {
    pub cited_text: String,
    pub end_block_index: u32,
    pub search_result_index: u32,
    pub source: String,
    pub start_block_index: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub title: Option<String>,
}

/// 🎯 @use: response-side citation variant — exactly one location type is active.
/// Used by: content, stream
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TextCitation {
    CharLocation(CitationCharLocation),
    PageLocation(CitationPageLocation),
    ContentBlockLocation(CitationContentBlockLocation),
    WebSearchResultLocation(CitationsWebSearchResultLocation),
    SearchResultLocation(CitationsSearchResultLocation),
}
