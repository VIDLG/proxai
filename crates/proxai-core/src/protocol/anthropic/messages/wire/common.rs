#![allow(
    dead_code,
    reason = "Anthropic Messages wire model includes protocol fields not yet observed by runtime summaries."
)]

use crate::protocol::RequiredNullable;
use crate::protocol::deserialize_present;
use serde::{Deserialize, Serialize};
use strum::Display;

// ── Primitive enums ───────────────────────────────────────────────────────

/// @sdk(shape = "StopReason")
/// @sdk(enum_extra = "model_context_window_exceeded", source = "BetaStopReason")
/// 🎯 @use: stop reason — explains why the model stopped generating.
/// Used by: message, stream
///
/// Read from `Message.stop_reason` on a response or from
/// `MessageDeltaEvent.delta.stop_reason` during streaming.
///
/// Flow: if "tool_use", process any ToolUseBlock(s) in the content array and
/// send results back; if "end_turn", the model has finished its response;
/// if "max_tokens", the response may be incomplete.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
    PauseTurn,
    Refusal,
    /// Defined by the official Beta `BetaStopReason` union and observed on the
    /// stable Messages stream in diagnostic
    /// `1784598362-1784598351961` (2026-07-21).
    ModelContextWindowExceeded,
}

/// @sdk(proxai_internal = "field_literal_wrapper")
/// response service tier — 'standard' | 'priority' | 'batch' | null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ResponseServiceTier {
    Standard,
    Priority,
    Batch,
}

/// @sdk(shape = "Container")
/// 🎯 @use: container reference — references a previously uploaded file container.
/// Used by: message, stream
///
/// Read from `ContentBlock::ContainerUpload(ContainerUploadBlock { file_id })`.
///
/// Use the file_id in subsequent ContainerUploadBlockParam requests.
///
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub expires_at: String,
}

// ── Cache types ───────────────────────────────────────────────────────────

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `CacheControlEphemeral.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheControlType {
    Ephemeral,
}

/// @sdk(proxai_internal = "field_literal_wrapper")
/// TTL value used by `CacheControlEphemeral.ttl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheControlTtl {
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "1h")]
    OneHour,
}

/// @sdk(shape = "CacheCreation")
/// cache creation stats — counts of input tokens cached at
/// different TTLs, returned as a sub-field of Usage.
///
/// Not directly constructed; read from `Usage.cache_creation` on a response.
///
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheCreation {
    pub ephemeral_1h_input_tokens: u32,
    pub ephemeral_5m_input_tokens: u32,
}

/// @sdk(shape = "CacheControlEphemeral")
/// 🎯 @use: ephemeral cache control marker applied to a content block.
/// Used by: bash, blocks, code_execution, content, request, search, text_editor, tool_use, tools, web
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheControlEphemeral {
    #[serde(rename = "type")]
    pub type_: CacheControlType,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub ttl: Option<CacheControlTtl>,
}

// ── Refusal types ─────────────────────────────────────────────────────────

/// @sdk(proxai_internal = "field_literal_wrapper")
/// Category value used by `RefusalStopDetails.category`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RefusalCategory {
    Cyber,
    Bio,
}

/// @sdk(proxai_internal = "discriminator")
/// Discriminator value used by `RefusalStopDetails.type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefusalStopDetailsType {
    Refusal,
}

/// @sdk(shape = "RefusalStopDetails")
/// 🎯 @use: refusal stop details — explains why the model refused to respond.
/// Used by: message, stream
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefusalStopDetails {
    #[serde(rename = "type")]
    pub type_: RefusalStopDetailsType,
    pub category: RequiredNullable<RefusalCategory>,
    pub explanation: RequiredNullable<String>,
}

// ── Usage ─────────────────────────────────────────────────────────────────

/// @sdk(shape = "ServerToolUsage")
/// 🎯 @use: server tool usage — counts for built-in tool invocations.
/// Used by: stream, self
///
/// Read from `Usage.server_tool_use` on a response.
///
/// Tracks how many web_search and web_fetch calls the server made on your behalf.
///
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerToolUsage {
    pub web_fetch_requests: u32,
    pub web_search_requests: u32,
}

/// @sdk(shape = "OutputTokensDetails")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputTokensDetails {
    pub thinking_tokens: u32,
}

/// @sdk(shape = "Usage")
/// 🎯 @use: input/output token usage summary — cost tracking and billing info.
/// Used by: message
///
/// Read from `Message.usage` on a response or from
/// `MessageStartEvent.message.usage` in streaming mode.
///
/// Use for cost tracking, billing, and debugging prompt size.
///
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub cache_creation: RequiredNullable<CacheCreation>,
    pub cache_creation_input_tokens: RequiredNullable<u32>,
    pub cache_read_input_tokens: RequiredNullable<u32>,
    pub inference_geo: RequiredNullable<String>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub output_tokens_details: RequiredNullable<OutputTokensDetails>,
    pub server_tool_use: RequiredNullable<ServerToolUsage>,
    pub service_tier: RequiredNullable<ResponseServiceTier>,
}
