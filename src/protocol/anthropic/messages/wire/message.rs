#![allow(
    dead_code,
    unused_imports,
    clippy::enum_variant_names,
    reason = "Anthropic Messages envelope schema mirrors upstream generated types."
)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    common::{Container, RefusalStopDetails, StopReason, Usage},
    content::{ContentBlock, ContentBlockParam},
};

/// Message.role: `'assistant'`.
/// MessageParam.role: `'user' | 'assistant'`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

// ── Leaf types ───────────────────────────────────────────────────────────

/// Message.type: `'message'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Message,
}

/// MessageParam.content: `string | Array<ContentBlockParam>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageParamContent {
    Text(String),
    Blocks(Vec<ContentBlockParam>),
}

// ── Message types ────────────────────────────────────────────────────────

/// 🎯 @use: message response.
/// Used by: stream
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    /// @sdk(required_nullable_accepts_missing)
    pub container: Option<Container>,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub role: Role,
    #[serde(rename = "type")]
    pub type_: MessageType,
    /// @sdk(required_nullable_accepts_missing)
    pub stop_details: Option<RefusalStopDetails>,
    /// @sdk(required_nullable_accepts_missing)
    pub stop_reason: Option<StopReason>,
    /// @sdk(required_nullable_accepts_missing)
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

/// 🎯 @use: request-side conversation message.
/// Used by: request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageParam {
    pub content: MessageParamContent,
    pub role: Role,
}
