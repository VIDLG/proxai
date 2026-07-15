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

use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::AsRefStr;

use super::citations::CitationsConfigParam;
use super::common::CacheControlEphemeral;

/// @sdk(proxai_internal = "field_literal_wrapper")
/// Tool.allowed_callers: `Array< 'direct' | 'code_execution_20250825' | 'code_execution_20260120' | 'code_execution_20260521' >`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowedCaller {
    Direct,
    CodeExecution20250825,
    CodeExecution20260120,
    CodeExecution20260521,
}

// Private imports for ToolUnion enum variants.

// ─────────────────────────────────────────────────────────────────────────────
// ── Custom tool definition ───────────────────────────────────────────────────

/// @sdk(shape = "InputSchema")
/// @sdk(field_suppress = "extra")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSchema {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub properties: OptionalNullable<Value>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub required: OptionalNullable<Vec<String>>,
    #[serde(flatten)]
    pub extra: Value,
}

impl Default for InputSchema {
    fn default() -> Self {
        Self {
            type_: "object".to_string(),
            properties: Some(serde_json::json!({})).into(),
            required: Some(Vec::new()).into(),
            extra: serde_json::json!({}),
        }
    }
}

/// @sdk(shape = "Tool")
/// A user-defined ("custom") function the model can call. Define the schema and description
/// to tell the model when and how to invoke the tool.
///
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tool {
    pub input_schema: InputSchema,
    pub name: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub allowed_callers: Option<Vec<AllowedCaller>>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub defer_loading: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub eager_input_streaming: OptionalNullable<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub input_examples: Option<Vec<Value>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub strict: Option<bool>,
    #[serde(
        rename = "type",
        default,
        skip_serializing_if = "OptionalNullable::is_missing"
    )]
    pub type_: OptionalNullable<String>,
}

// ── Server tool definition (unified) ────────────────────────────────────────

/// Unified struct merged from SDK's per-version tool interfaces
/// (ToolBash20250124, CodeExecutionTool20250522, etc.).
/// @sdk(alias = "ToolBash20250124")
/// @sdk(alias = "CodeExecutionTool20250522")
/// @sdk(alias = "CodeExecutionTool20250825")
/// @sdk(alias = "CodeExecutionTool20260120")
/// @sdk(alias = "CodeExecutionTool20260521")
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

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `UserLocation.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApproximateType {
    Approximate,
}

/// @sdk(proxai_internal = "field_literal_wrapper")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseInclusion {
    Full,
    Excluded,
}

/// @sdk(shape = "UserLocation")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserLocation {
    #[serde(rename = "type")]
    pub type_: ApproximateType,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub city: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub country: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub region: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub timezone: OptionalNullable<String>,
}

/// Unified struct for all web search/fetch tool versions.
/// @sdk(alias = "WebSearchTool20250305")
/// @sdk(alias = "WebSearchTool20260209")
/// @sdk(alias = "WebFetchTool20250910")
/// @sdk(alias = "WebFetchTool20260209")
/// @sdk(alias = "WebFetchTool20260309")
/// @sdk(alias = "WebFetchTool20260318")
/// @sdk(alias = "WebSearchTool20260318")
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_inclusion: Option<ResponseInclusion>,
}

/// @sdk(shape = "ToolUnion")
/// 🎯 @use: union of all built-in and custom tool definitions supported by the API.
/// Used by: request
///
/// Use this when specifying the `tools` field in a create-message request.
/// Each variant corresponds to a different tool type with its own versioned
/// schema.
///
/// @sdk(union_variant = "Tool", rust = "Custom")
#[derive(Debug, Clone, PartialEq, Eq, AsRefStr, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolUnion {
    #[strum(serialize = "custom")]
    #[serde(rename = "custom")]
    Custom(Tool),
    #[strum(serialize = "bash_20250124")]
    #[serde(rename = "bash_20250124")]
    ToolBash20250124(ServerToolDef),
    #[strum(serialize = "code_execution_20250522")]
    #[serde(rename = "code_execution_20250522")]
    CodeExecutionTool20250522(ServerToolDef),
    #[strum(serialize = "code_execution_20250825")]
    #[serde(rename = "code_execution_20250825")]
    CodeExecutionTool20250825(ServerToolDef),
    #[strum(serialize = "code_execution_20260120")]
    #[serde(rename = "code_execution_20260120")]
    CodeExecutionTool20260120(ServerToolDef),
    #[strum(serialize = "code_execution_20260521")]
    #[serde(rename = "code_execution_20260521")]
    CodeExecutionTool20260521(ServerToolDef),
    #[strum(serialize = "memory_20250818")]
    #[serde(rename = "memory_20250818")]
    MemoryTool20250818(ServerToolDef),
    #[strum(serialize = "text_editor_20250124")]
    #[serde(rename = "text_editor_20250124")]
    ToolTextEditor20250124(ServerToolDef),
    #[strum(serialize = "text_editor_20250429")]
    #[serde(rename = "text_editor_20250429")]
    ToolTextEditor20250429(ServerToolDef),
    #[strum(serialize = "text_editor_20250728")]
    #[serde(rename = "text_editor_20250728")]
    ToolTextEditor20250728(ServerToolDef),
    #[strum(serialize = "web_search_20250305")]
    #[serde(rename = "web_search_20250305")]
    WebSearchTool20250305(WebToolDef),
    #[strum(serialize = "web_fetch_20250910")]
    #[serde(rename = "web_fetch_20250910")]
    WebFetchTool20250910(WebToolDef),
    #[strum(serialize = "web_search_20260209")]
    #[serde(rename = "web_search_20260209")]
    WebSearchTool20260209(WebToolDef),
    #[strum(serialize = "web_fetch_20260209")]
    #[serde(rename = "web_fetch_20260209")]
    WebFetchTool20260209(WebToolDef),
    #[strum(serialize = "web_fetch_20260309")]
    #[serde(rename = "web_fetch_20260309")]
    WebFetchTool20260309(WebToolDef),
    #[strum(serialize = "web_fetch_20260318")]
    #[serde(rename = "web_fetch_20260318")]
    WebFetchTool20260318(WebToolDef),
    #[strum(serialize = "web_search_20260318")]
    #[serde(rename = "web_search_20260318")]
    WebSearchTool20260318(WebToolDef),
    #[strum(serialize = "tool_search_tool_bm25_20251119")]
    #[serde(rename = "tool_search_tool_bm25_20251119")]
    ToolSearchToolBm25_20251119(ServerToolDef),
    #[strum(serialize = "tool_search_tool_regex_20251119")]
    #[serde(rename = "tool_search_tool_regex_20251119")]
    ToolSearchToolRegex20251119(ServerToolDef),
}
