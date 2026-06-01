#![allow(
    dead_code,
    reason = "Anthropic Messages tool wire model includes variants reserved for protocol coverage and translation."
)]

pub mod bash;
pub mod code_execution;
pub mod search;
pub mod text_editor;
pub mod tool_use;
pub mod web;

pub use bash::*;
pub use code_execution::*;
pub use search::*;
pub use text_editor::*;
pub use tool_use::*;
pub use web::*;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::citations::CitationsConfigParam;
use super::common::CacheControlEphemeral;

/// Tool.allowed_callers: `Array<'direct' | 'code_execution_20250825' | 'code_execution_20260120'>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowedCaller {
    Direct,
    CodeExecution20250825,
    CodeExecution20260120,
}

// Private imports for ToolUnion enum variants.

// ─────────────────────────────────────────────────────────────────────────────
// ── Custom tool definition ───────────────────────────────────────────────────

/// @sdk(field_suppress = "extra")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSchema {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: Value,
}

/// A user-defined ("custom") function the model can call. Define the schema and description
/// to tell the model when and how to invoke the tool.
///
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tool {
    pub input_schema: InputSchema,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eager_input_streaming: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_examples: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
}

// ── Server tool definition (unified) ────────────────────────────────────────

/// Unified struct merged from SDK's per-version tool interfaces
/// (ToolBash20250124, CodeExecutionTool20250522, etc.).
/// @sdk(alias = "ToolBash20250124")
/// @sdk(alias = "CodeExecutionTool20250522")
/// @sdk(alias = "CodeExecutionTool20250825")
/// @sdk(alias = "CodeExecutionTool20260120")
/// @sdk(alias = "MemoryTool20250818")
/// @sdk(alias = "ToolTextEditor20250124")
/// @sdk(alias = "ToolTextEditor20250429")
/// @sdk(alias = "ToolTextEditor20250728")
/// @sdk(alias = "ToolSearchToolBm25_20251119")
/// @sdk(alias = "ToolSearchToolRegex20251119")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerToolDef {
    #[serde(rename = "name", skip_deserializing)]
    pub name: String,
    #[serde(rename = "type", skip_deserializing)]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_examples: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_characters: Option<u32>,
}

// ── Shared types for web tool definitions ─────────────────────────────────

/// Discriminator value used by `UserLocation.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApproximateType {
    Approximate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserLocation {
    #[serde(rename = "type")]
    pub type_: ApproximateType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

/// Unified struct for all web search/fetch tool versions.
/// @sdk(alias = "WebSearchTool20250305")
/// @sdk(alias = "WebSearchTool20260209")
/// @sdk(alias = "WebFetchTool20250910")
/// @sdk(alias = "WebFetchTool20260209")
/// @sdk(alias = "WebFetchTool20260309")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebToolDef {
    #[serde(rename = "name", skip_deserializing)]
    pub name: String,
    #[serde(rename = "type", skip_deserializing)]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_location: Option<UserLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<CitationsConfigParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_content_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_cache: Option<bool>,
}

/// 🎯 @use: union of all built-in and custom tool definitions supported by the API.
/// Used by: request
///
/// Use this when specifying the `tools` field in a create-message request.
/// Each variant corresponds to a different tool type with its own versioned
/// schema.
///
/// @sdk(union_variant = "Tool", rust = "Custom")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolUnion {
    #[serde(rename = "custom")]
    Custom(Tool),
    #[serde(rename = "bash_20250124")]
    ToolBash20250124(ServerToolDef),
    #[serde(rename = "code_execution_20250522")]
    CodeExecutionTool20250522(ServerToolDef),
    #[serde(rename = "code_execution_20250825")]
    CodeExecutionTool20250825(ServerToolDef),
    #[serde(rename = "code_execution_20260120")]
    CodeExecutionTool20260120(ServerToolDef),
    #[serde(rename = "memory_20250818")]
    MemoryTool20250818(ServerToolDef),
    #[serde(rename = "text_editor_20250124")]
    ToolTextEditor20250124(ServerToolDef),
    #[serde(rename = "text_editor_20250429")]
    ToolTextEditor20250429(ServerToolDef),
    #[serde(rename = "text_editor_20250728")]
    ToolTextEditor20250728(ServerToolDef),
    #[serde(rename = "web_search_20250305")]
    WebSearchTool20250305(WebToolDef),
    #[serde(rename = "web_fetch_20250910")]
    WebFetchTool20250910(WebToolDef),
    #[serde(rename = "web_search_20260209")]
    WebSearchTool20260209(WebToolDef),
    #[serde(rename = "web_fetch_20260209")]
    WebFetchTool20260209(WebToolDef),
    #[serde(rename = "web_fetch_20260309")]
    WebFetchTool20260309(WebToolDef),
    #[serde(rename = "tool_search_tool_bm25_20251119")]
    ToolSearchToolBm25_20251119(ServerToolDef),
    #[serde(rename = "tool_search_tool_regex_20251119")]
    ToolSearchToolRegex20251119(ServerToolDef),
}
