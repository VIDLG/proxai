#![allow(
    dead_code,
    reason = "Anthropic Messages tool-use block schema mirrors upstream generated types."
)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::Display;

use super::super::common::CacheControlEphemeral;

// ── Caller identity types ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectCaller;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerToolCaller {
    pub tool_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerToolCaller20260120 {
    pub tool_id: String,
}

/// 🎯 @use: caller identity — discriminator for Direct/Server-tool callers.
/// Used by: web, self
/// ToolUseBlock.caller: `DirectCaller | ServerToolCaller | ServerToolCaller20260120`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolCaller {
    #[serde(rename = "direct")]
    Direct(DirectCaller),
    #[serde(rename = "code_execution_20250825")]
    CodeExecution20250825(ServerToolCaller),
    #[serde(rename = "code_execution_20260120")]
    CodeExecution20260120(ServerToolCaller20260120),
}

// ── Server tool name ───────────────────────────────────────────────────────

/// ServerToolUseBlock.name:
///   `'web_search' | 'web_fetch' | 'code_execution' | 'bash_code_execution' | 'text_editor_code_execution' | 'tool_search_tool_regex' | 'tool_search_tool_bm25'`.
#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ServerToolName {
    WebSearch,
    WebFetch,
    CodeExecution,
    BashCodeExecution,
    TextEditorCodeExecution,
    ToolSearchToolRegex,
    ToolSearchToolBm25,
}

// ── Response-side tool blocks ──────────────────────────────────────────────

/// 🎯 @use: tool-use block — the model decided to call a tool.
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolUseBlock {
    pub id: String,
    pub caller: ToolCaller,
    pub input: Value,
    pub name: String,
}

/// 🎯 @use: server-tool-use block — the server decided to call a built-in tool
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerToolUseBlock {
    pub id: String,
    pub caller: ToolCaller,
    pub input: Value,
    pub name: ServerToolName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolReferenceBlock {
    pub tool_name: String,
    #[serde(rename = "type")]
    pub type_: String,
}

// ── Request-side tool block params ─────────────────────────────────────────

/// 🎯 @use: tool-use param — forwards a previously received `ToolUseBlock`
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolUseBlockParam {
    pub id: String,
    pub input: Value,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller: Option<ToolCaller>,
}

/// 🎯 @use: server-tool-use param — forwards a previously received
/// Used by: content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerToolUseBlockParam {
    pub id: String,
    pub input: Value,
    pub name: ServerToolName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller: Option<ToolCaller>,
}

/// 🎯 @use: tool-reference param — references a tool defined in an earlier
/// Used by: content, search
/// turn's `tools` array.
///
/// Constructed via `ContentBlockParam::ToolReference(ToolReferenceBlockParam { .. })`.
///
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolReferenceBlockParam {
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
}

/// Proxai accepts the `ToolResultBlockParam` content subset here.
/// @sdk(proxai_internal = "projection")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolResultBlock {
    pub tool_use_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// 🎯 @use: tool-result param
/// Used by: content
///
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolResultBlockParam {
    pub tool_use_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
}
