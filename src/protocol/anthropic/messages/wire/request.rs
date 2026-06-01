#![allow(
    dead_code,
    reason = "Anthropic Messages request wire model includes fields reserved for protocol coverage and translation."
)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    blocks::TextBlockType, citations::TextCitationParam, common::CacheControlEphemeral,
    message::MessageParam, tools::ToolUnion,
};

// ── Leaf type aliases ─────────────────────────────────────────────────────

pub type Model = String;

pub type MessageCountTokensTool = ToolUnion;

// ── Thinking config types ────────────────────────────────────────────────

/// ThinkingConfigEnabled.display: `'summarized' | 'omitted' | null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingDisplay {
    Summarized,
    Omitted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingConfigEnabled {
    pub budget_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<ThinkingDisplay>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingConfigDisabled;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingConfigAdaptive {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<ThinkingDisplay>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ThinkingConfigParam {
    #[serde(rename = "enabled")]
    Enabled(ThinkingConfigEnabled),
    #[serde(rename = "disabled")]
    Disabled(ThinkingConfigDisabled),
    #[serde(rename = "adaptive")]
    Adaptive(ThinkingConfigAdaptive),
}

// ── Output config types ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonOutputFormat {
    pub schema: Value,
}

/// OutputConfig.effort: `'low' | 'medium' | 'high' | 'xhigh' | 'max' | null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputEffort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

/// OutputConfig.format wrapper for `JSONOutputFormat`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutputFormat {
    #[serde(rename = "json_schema")]
    JsonSchema(JsonOutputFormat),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<OutputEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<OutputFormat>,
}

// ── System prompt types ──────────────────────────────────────────────────

/// @sdk(shape = "TextBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedTextBlockParam {
    #[serde(rename = "type")]
    pub type_: TextBlockType,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Vec<TextCitationParam>>,
}

/// MessageCreateParamsBase.system: `string | Array<TextBlockParam>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemPrompt {
    Text(String),
    Blocks(Vec<TypedTextBlockParam>),
}

// ── Message token types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageTokensCount {
    pub input_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageCountTokensParams {
    pub messages: Vec<MessageParam>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<OutputConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfigParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolUnion>>,
}

// ── Tool choice types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceAuto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_parallel_tool_use: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceAny {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_parallel_tool_use: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_parallel_tool_use: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceNone;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolChoice {
    #[serde(rename = "auto")]
    Auto(ToolChoiceAuto),
    #[serde(rename = "any")]
    Any(ToolChoiceAny),
    #[serde(rename = "tool")]
    Tool(ToolChoiceTool),
    #[serde(rename = "none")]
    None(ToolChoiceNone),
}

// ── Request metadata ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

// ── Message create params ────────────────────────────────────────────────

/// MessageCreateParamsBase.service_tier: `'auto' | 'standard_only'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestServiceTier {
    Auto,
    StandardOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageCreateParamsBase {
    pub max_tokens: u32,
    pub messages: Vec<MessageParam>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_geo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<OutputConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<RequestServiceTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<serde_json::Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfigParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolUnion>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<serde_json::Number>,
}

/// @sdk(internal = "MessageCreateParams")
/// @sdk(internal = "MessageStreamParams")
/// @sdk(field_suppress = "stream")
/// @sdk(field_suppress = "base")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageCreateParamsNonStreaming {
    #[serde(flatten)]
    pub base: MessageCreateParamsBase,
}

/// @sdk(field_suppress = "stream")
/// @sdk(field_suppress = "base")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageCreateParamsStreaming {
    #[serde(flatten)]
    pub base: MessageCreateParamsBase,
}
