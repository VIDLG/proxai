#![allow(
    dead_code,
    reason = "Anthropic Messages request wire model includes fields reserved for protocol coverage and translation."
)]

use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    blocks::TextBlockType, citations::TextCitationParam, common::CacheControlEphemeral,
    message::MessageParam, tools::ToolUnion,
};

// ── Leaf type aliases ─────────────────────────────────────────────────────

/// @sdk(shape = "Model")
pub type Model = String;

/// @sdk(shape = "MessageCountTokensTool")
pub type MessageCountTokensTool = ToolUnion;

// ── Thinking config types ────────────────────────────────────────────────

/// @sdk(proxai_internal = "field_literal_wrapper")
/// ThinkingConfigEnabled.display: `'summarized' | 'omitted' | null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingDisplay {
    Summarized,
    Omitted,
}

/// @sdk(shape = "ThinkingConfigEnabled")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingConfigEnabled {
    pub budget_tokens: u32,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub display: OptionalNullable<ThinkingDisplay>,
}

/// @sdk(shape = "ThinkingConfigDisabled")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingConfigDisabled;

/// @sdk(shape = "ThinkingConfigAdaptive")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingConfigAdaptive {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub display: OptionalNullable<ThinkingDisplay>,
}

/// @sdk(shape = "ThinkingConfigParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ThinkingConfigParam {
    Enabled(ThinkingConfigEnabled),
    Disabled(ThinkingConfigDisabled),
    Adaptive(ThinkingConfigAdaptive),
}

// ── Output config types ──────────────────────────────────────────────────

/// @sdk(shape = "JSONOutputFormat")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonOutputFormat {
    pub schema: Value,
}

/// @sdk(proxai_internal = "field_literal_wrapper")
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
/// @sdk(proxai_internal = "union_wrapper")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputFormat {
    JsonSchema(JsonOutputFormat),
}

/// @sdk(shape = "OutputConfig")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub effort: OptionalNullable<OutputEffort>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub format: OptionalNullable<OutputFormat>,
}

// ── System prompt types ──────────────────────────────────────────────────

/// @sdk(shape = "TextBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedTextBlockParam {
    #[serde(rename = "type")]
    pub type_: TextBlockType,
    pub text: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub citations: OptionalNullable<Vec<TextCitationParam>>,
}

/// @sdk(proxai_internal = "union_wrapper")
/// MessageCreateParamsBase.system: `string | Array<TextBlockParam>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemPrompt {
    Text(String),
    Blocks(Vec<TypedTextBlockParam>),
}

// ── Message token types ──────────────────────────────────────────────────

/// @sdk(shape = "MessageTokensCount")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageTokensCount {
    pub input_tokens: u32,
}

/// @sdk(shape = "MessageCountTokensParams")
/// @sdk(field_suppress = "user_profile_id")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageCountTokensParams {
    pub messages: Vec<MessageParam>,
    pub model: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub output_config: Option<OutputConfig>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub system: Option<SystemPrompt>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub thinking: Option<ThinkingConfigParam>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub tool_choice: Option<ToolChoice>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub tools: Option<Vec<ToolUnion>>,
}

// ── Tool choice types ──────────────────────────────────────────────────────

/// @sdk(shape = "ToolChoiceAuto")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceAuto {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub disable_parallel_tool_use: Option<bool>,
}

/// @sdk(shape = "ToolChoiceAny")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceAny {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub disable_parallel_tool_use: Option<bool>,
}

/// @sdk(shape = "ToolChoiceTool")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceTool {
    pub name: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub disable_parallel_tool_use: Option<bool>,
}

/// @sdk(shape = "ToolChoiceNone")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceNone;

/// @sdk(shape = "ToolChoice")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    Auto(ToolChoiceAuto),
    Any(ToolChoiceAny),
    Tool(ToolChoiceTool),
    None(ToolChoiceNone),
}

// ── Request metadata ─────────────────────────────────────────────────────

/// @sdk(shape = "Metadata")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub user_id: OptionalNullable<String>,
}

// ── Message create params ────────────────────────────────────────────────

/// @sdk(proxai_internal = "field_literal_wrapper")
/// MessageCreateParamsBase.service_tier: `'auto' | 'standard_only'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestServiceTier {
    Auto,
    StandardOnly,
}

/// @sdk(shape = "MessageCreateParamsBase")
/// @sdk(field_suppress = "user_profile_id")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageCreateParamsBase {
    pub max_tokens: u32,
    pub messages: Vec<MessageParam>,
    pub model: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub container: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub inference_geo: OptionalNullable<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub metadata: Option<Metadata>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub output_config: Option<OutputConfig>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub service_tier: Option<RequestServiceTier>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub stream: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub system: Option<SystemPrompt>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub temperature: Option<serde_json::Number>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub thinking: Option<ThinkingConfigParam>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub tool_choice: Option<ToolChoice>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub tools: Option<Vec<ToolUnion>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub top_k: Option<u32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub top_p: Option<serde_json::Number>,
}

/// @sdk(shape = "MessageCreateParamsNonStreaming")
/// @sdk(internal = "MessageCreateParams")
/// @sdk(internal = "MessageStreamParams")
/// @sdk(field_suppress = "stream")
/// @sdk(field_suppress = "base")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageCreateParamsNonStreaming {
    #[serde(flatten)]
    pub base: MessageCreateParamsBase,
}

/// @sdk(shape = "MessageCreateParamsStreaming")
/// @sdk(field_suppress = "stream")
/// @sdk(field_suppress = "base")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageCreateParamsStreaming {
    #[serde(flatten)]
    pub base: MessageCreateParamsBase,
}
