#![allow(
    dead_code,
    unused_imports,
    clippy::enum_variant_names,
    reason = "Anthropic Messages envelope schema mirrors upstream generated types."
)]

use crate::protocol::RequiredNullable;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    common::{Container, RefusalStopDetails, StopReason, Usage},
    content::{ContentBlock, ContentBlockParam},
};

/// @sdk(proxai_internal = "field_literal_wrapper")
/// Message.role: `'assistant'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    Assistant,
}

/// @sdk(proxai_internal = "field_literal_wrapper")
/// MessageParam.role: `'user' | 'assistant' | 'system'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageParamRole {
    User,
    Assistant,
    System,
}

// ── Leaf types ───────────────────────────────────────────────────────────

/// @sdk(proxai_internal = "discriminator")
/// Message.type: `'message'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Message,
}

/// @sdk(proxai_internal = "union_wrapper")
/// MessageParam.content: `string | Array<ContentBlockParam>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageParamContent {
    Text(String),
    Blocks(Vec<ContentBlockParam>),
}

// ── Message types ────────────────────────────────────────────────────────

/// @sdk(shape = "Message")
/// 🎯 @use: message response.
/// Used by: stream
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub container: RequiredNullable<Container>,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub role: MessageRole,
    #[serde(rename = "type")]
    pub type_: MessageType,
    pub stop_details: RequiredNullable<RefusalStopDetails>,
    pub stop_reason: RequiredNullable<StopReason>,
    pub stop_sequence: RequiredNullable<String>,
    pub usage: Usage,
}

/// @sdk(shape = "MessageParam")
/// 🎯 @use: request-side conversation message.
/// Used by: request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageParam {
    pub content: MessageParamContent,
    pub role: MessageParamRole,
}
