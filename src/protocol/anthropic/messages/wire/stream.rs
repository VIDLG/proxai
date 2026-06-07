#![allow(
    dead_code,
    clippy::enum_variant_names,
    reason = "Anthropic Messages stream wire model mirrors upstream names beyond currently observed runtime summaries."
)]

//! Anthropic Messages SSE stream wire types.
//!
//! A typical `/v1/messages` stream starts with `message_start`, then emits
//! zero or more content block lifecycles:
//! `content_block_start` -> `content_block_delta` -> `content_block_stop`.
//! Near the end, `message_delta` carries message-level stop state and usage,
//! followed by `message_stop`. `ping` events may appear between semantic
//! events as connection heartbeats.

use serde::{Deserialize, Serialize};

use super::{
    citations::TextCitation,
    common::{Container, OutputTokensDetails, RefusalStopDetails, ServerToolUsage, StopReason},
    content::ContentBlock,
    message::Message,
};

/// Text chunk emitted while streaming a `text` content block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextDelta {
    pub text: String,
}

/// Partial JSON fragment emitted while streaming a tool input object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputJsonDelta {
    pub partial_json: String,
}

/// Citation emitted after text has streamed for a citeable content block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationsDelta {
    pub citation: TextCitation,
}

/// Thinking text chunk emitted when extended thinking is enabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingDelta {
    pub thinking: String,
}

/// Signature emitted after streamed thinking content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureDelta {
    pub signature: String,
}

/// Payload union for `content_block_delta` SSE events.
/// Flow: emitted zero or more times between `content_block_start` and
/// `content_block_stop` for the same content block index.
/// @sdk(shape = "RawContentBlockDelta")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlockDelta {
    #[serde(rename = "text_delta")]
    TextDelta(TextDelta),
    #[serde(rename = "input_json_delta")]
    InputJsonDelta(InputJsonDelta),
    #[serde(rename = "citations_delta")]
    CitationsDelta(CitationsDelta),
    #[serde(rename = "thinking_delta")]
    ThinkingDelta(ThinkingDelta),
    #[serde(rename = "signature_delta")]
    SignatureDelta(SignatureDelta),
}

/// SSE event carrying an incremental update for one content block.
/// Flow: updates the block identified by `index`; text, tool input JSON,
/// citations, thinking, and signatures use different delta payloads.
/// @sdk(shape = "RawContentBlockDeltaEvent")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentBlockDeltaEvent {
    pub delta: ContentBlockDelta,
    pub index: u32,
}

/// SSE event that opens a content block at a stable stream index.
/// Flow: carries a typed `ContentBlock` shell from the SDK shape; streaming
/// fields such as text and tool input may be incomplete until later
/// `content_block_delta` events and `content_block_stop`.
/// @sdk(shape = "RawContentBlockStartEvent")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentBlockStartEvent {
    pub content_block: ContentBlock,
    pub index: u32,
}

/// SSE heartbeat event.
/// Payload: {"type": "ping"}
/// Flow: may appear between semantic stream events to keep the connection live.
/// @sdk(proxai_internal = "stream-event")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PingEvent {}

/// SSE event that closes a content block at a stable stream index.
/// Flow: follows the final `content_block_delta` for the same `index`.
/// @sdk(shape = "RawContentBlockStopEvent")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentBlockStopEvent {
    pub index: u32,
}

/// Message-level delta emitted near the end of a stream.
/// Flow: carries stop reason, stop details, stop sequence, and container
/// updates that apply to the overall message rather than one content block.
/// @sdk(shape = "Delta")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDelta {
    /// @sdk(required_nullable_accepts_missing)
    pub container: Option<Container>,
    /// @sdk(required_nullable_accepts_missing)
    pub stop_details: Option<RefusalStopDetails>,
    /// @sdk(required_nullable_accepts_missing)
    pub stop_reason: Option<StopReason>,
    /// @sdk(required_nullable_accepts_missing)
    pub stop_sequence: Option<String>,
}

/// Token usage update emitted with a message-level stream delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDeltaUsage {
    /// @sdk(required_nullable_accepts_missing)
    pub cache_creation_input_tokens: Option<u32>,
    /// @sdk(required_nullable_accepts_missing)
    pub cache_read_input_tokens: Option<u32>,
    /// @sdk(required_nullable_accepts_missing)
    pub input_tokens: Option<u32>,
    pub output_tokens: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub output_tokens_details: Option<OutputTokensDetails>,
    /// @sdk(required_nullable_accepts_missing)
    pub server_tool_use: Option<ServerToolUsage>,
}

/// SSE event carrying message-level delta and usage updates.
/// Flow: usually appears after content block events and before `message_stop`;
/// carries stop state and usage for the whole message.
/// @sdk(shape = "RawMessageDeltaEvent")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDeltaEvent {
    pub delta: MessageDelta,
    pub usage: MessageDeltaUsage,
}

/// First semantic SSE event for a streamed response.
/// Flow: carries the initial `Message` envelope before content block events.
/// @sdk(shape = "RawMessageStartEvent")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageStartEvent {
    pub message: Message,
}

/// Final semantic SSE event for a streamed response.
/// Flow: marks that no further Anthropic Messages stream events are expected.
/// @sdk(shape = "RawMessageStopEvent")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageStopEvent;

/// Raw Anthropic Messages SSE event union.
/// Flow: deserializes each `data:` line from a real `text/event-stream`
/// `/v1/messages` response in the live protocol test.
/// @sdk(shape = "RawMessageStreamEvent")
/// Note: SDK stream parsers may handle ping out of band. We include it
/// explicitly here because proxai models the raw SSE payload shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageStreamEvent {
    #[serde(rename = "ping")]
    Ping(PingEvent),
    #[serde(rename = "message_start")]
    MessageStart(MessageStartEvent),
    #[serde(rename = "message_delta")]
    MessageDelta(MessageDeltaEvent),
    #[serde(rename = "message_stop")]
    MessageStop(MessageStopEvent),
    #[serde(rename = "content_block_start")]
    ContentBlockStart(ContentBlockStartEvent),
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta(ContentBlockDeltaEvent),
    #[serde(rename = "content_block_stop")]
    ContentBlockStop(ContentBlockStopEvent),
}
