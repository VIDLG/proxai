//! Per-block accumulation state for
//! `anthropic_messages -> openai_responses` streaming translation.
//!
//! Holds the per-message accumulator (`StreamingState`) and per-content-block
//! slots (`StreamBlock`). All registration, accumulation, snapshot, and
//! terminal-status logic lives here; the translator in `super::mod` only
//! drives these methods in response to inbound Anthropic events.

use std::collections::BTreeMap;

use crate::protocol::anthropic::messages::{StopReason, TextCitation, Usage};
use crate::protocol::openai_responses::{OutputItem, Response, ResponseUsage, Status};
use crate::translation::streaming::{
    StreamIdentity, StreamTranslationError, StreamTranslationResult,
};

use super::super::ids::OutputItemIdAllocator;
use super::super::types::incomplete_details_from_stop_reason;

#[derive(Debug)]
pub(super) struct StreamingState {
    pub(super) usage: Usage,
    item_ids: OutputItemIdAllocator,
    pub(super) stop_reason: Option<StopReason>,
    blocks: BTreeMap<u32, StreamBlock>,
    pub(super) output_items: Vec<OutputItem>,
    // Cumulative character count of all completed text items so far. Used as
    // the base offset when translating a TextBlock's citations to Responses
    // annotations, which use character indices relative to the full text
    // output (matching the non-streaming path in response.rs).
    pub(super) text_char_offset: usize,
}

/// Per-block in-flight state, keyed by Anthropic `content_block.index` in
/// `StreamingState::blocks`.
///
/// Each variant's fields split into two roles:
///
/// - **Accumulated content** (`text`, `arguments`): initialized empty by
///   `register_*_block`, filled by the matching `append_*_delta` as
///   `content_block_delta` events arrive. Read at `content_block_stop` time
///   to build the finalized `OutputItem`.
/// - **One-shot metadata** (`item_id`, `citations`, `name`, `data`): attached
///   once at registration, never mutated, consumed at stop time. These are
///   fields whose Anthropic-side values arrive in full on `content_block_start`
///   and have no delta equivalent proxai supports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum StreamBlock {
    Text {
        item_id: String,
        text: String,
        citations: Option<Vec<TextCitation>>,
    },
    Thinking {
        item_id: String,
        text: String,
    },
    RedactedThinking {
        item_id: String,
        data: String,
    },
    ToolUse {
        item_id: String,
        name: String,
        arguments: String,
    },
}

impl StreamingState {
    pub(super) fn new(identity: &StreamIdentity, usage: Usage) -> Self {
        let item_ids = OutputItemIdAllocator::new(identity.id().to_string());
        Self {
            usage,
            item_ids,
            stop_reason: None,
            blocks: BTreeMap::new(),
            output_items: Vec::new(),
            text_char_offset: 0,
        }
    }

    pub(super) fn next_message_item_id(&mut self) -> String {
        self.item_ids.message()
    }

    pub(super) fn next_reasoning_item_id(&mut self) -> String {
        self.item_ids.reasoning()
    }

    pub(super) fn terminal_response_status(&self) -> Status {
        // Match the non-streaming conversion in types.rs: refusal is Failed,
        // max_tokens is Incomplete, everything else is Completed. Reusing the
        // shared `From<StopReason> for Status` impl keeps streaming and
        // non-streaming terminal status in lockstep.
        self.stop_reason
            .map(Status::from)
            .unwrap_or(Status::Completed)
    }

    pub(super) fn response_snapshot(&self, identity: &StreamIdentity, status: Status) -> Response {
        let incomplete_details = incomplete_details_from_stop_reason(self.stop_reason);
        Response {
            background: None,
            billing: None,
            conversation: None,
            created_at: 0,
            completed_at: None,
            error: None,
            id: identity.id().to_string(),
            incomplete_details,
            instructions: None,
            max_output_tokens: None,
            metadata: None,
            model: identity.model().to_string(),
            object: "response".to_string(),
            output: self.output_items.clone(),
            parallel_tool_calls: None,
            previous_response_id: None,
            prompt: None,
            prompt_cache_key: None,
            prompt_cache_retention: None,
            reasoning: None,
            safety_identifier: None,
            service_tier: self.usage.service_tier.and_then(Into::into),
            status,
            temperature: None,
            text: None,
            tool_choice: None,
            tools: None,
            top_logprobs: None,
            top_p: None,
            truncation: None,
            usage: Some(ResponseUsage::from(&self.usage)),
        }
    }

    /// Open a text block slot.
    ///
    /// `text` is intentionally initialized to an empty string. Anthropic
    /// delivers text incrementally via `content_block_delta.text_delta`
    /// events (and may also send an initial non-empty `text` on
    /// `content_block_start`). All text — including any text present on
    /// `content_block_start` — is accumulated via `append_text_delta`, which
    /// keeps the start/delta paths uniform and avoids duplicating push_str
    /// logic here.
    ///
    /// `citations` is attached directly because, in the wire shape proxai
    /// supports, citations arrive once and in full on `content_block_start`
    /// (the `citations_delta` event variant is rejected as unrepresentable).
    /// They never change after registration, so they live as block metadata
    /// rather than as accumulated content. They are consumed at
    /// `content_block_stop` time by `text_block_annotations`.
    ///
    /// `mark_text` is NOT called here: `content_block_start.text` may be
    /// empty, and the representable-output tracker must only flip when the
    /// stream has actually produced Responses-representable text. The caller
    /// marks the phase output when appending a non-empty text delta.
    pub(super) fn register_text_block(
        &mut self,
        block_index: u32,
        item_id: String,
        citations: Option<Vec<TextCitation>>,
    ) -> StreamTranslationResult<()> {
        self.register_block(
            block_index,
            StreamBlock::Text {
                item_id,
                text: String::new(),
                citations,
            },
        )
    }

    /// Open a thinking block slot. See `register_text_block` for why `text`
    /// starts empty and why `mark_reasoning` is deferred until the caller
    /// appends a non-empty thinking delta.
    pub(super) fn register_thinking_block(
        &mut self,
        block_index: u32,
        item_id: String,
    ) -> StreamTranslationResult<()> {
        self.register_block(
            block_index,
            StreamBlock::Thinking {
                item_id,
                text: String::new(),
            },
        )
    }

    /// Open a redacted-thinking block slot and immediately mark reasoning as
    /// emitted.
    ///
    /// Unlike text/thinking, redacted thinking has no streamed delta events
    /// of any kind in the Anthropic wire model — the entire opaque `data`
    /// blob arrives on `content_block_start` and never changes afterwards.
    /// There is therefore no `append_redacted_thinking_delta` companion, and
    /// `data` is attached here rather than accumulated. Because the payload
    /// The caller marks reasoning immediately after registration because the
    /// payload is non-empty by construction at registration time.
    pub(super) fn register_redacted_thinking_block(
        &mut self,
        block_index: u32,
        item_id: String,
        data: String,
    ) -> StreamTranslationResult<()> {
        self.register_block(block_index, StreamBlock::RedactedThinking { item_id, data })?;
        Ok(())
    }

    /// Open a tool-use block slot and immediately mark tool use as emitted.
    ///
    /// `arguments` starts empty: the JSON argument string is built up by
    /// subsequent `input_json_delta` events via `append_tool_arguments_delta`.
    ///
    /// The caller marks tool use immediately after registration because `id`
    /// and `name` are mandatory and always present on `content_block_start`.
    pub(super) fn register_tool_use_block(
        &mut self,
        block_index: u32,
        item_id: String,
        name: String,
    ) -> StreamTranslationResult<()> {
        self.register_block(
            block_index,
            StreamBlock::ToolUse {
                item_id,
                name,
                arguments: String::new(),
            },
        )?;
        Ok(())
    }

    fn register_block(
        &mut self,
        block_index: u32,
        block: StreamBlock,
    ) -> StreamTranslationResult<()> {
        if self.blocks.insert(block_index, block).is_some() {
            return Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted duplicate content_block_start index {block_index}"
            )));
        }
        Ok(())
    }

    pub(super) fn append_text_delta(
        &mut self,
        block_index: u32,
        delta: &str,
    ) -> StreamTranslationResult<String> {
        match self.blocks.get_mut(&block_index) {
            Some(StreamBlock::Text { item_id, text, .. }) => {
                text.push_str(delta);
                Ok(item_id.clone())
            }
            Some(
                StreamBlock::Thinking { .. }
                | StreamBlock::RedactedThinking { .. }
                | StreamBlock::ToolUse { .. },
            ) => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted text_delta for incompatible content block index {block_index}"
            ))),
            None => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted text_delta for unopened content block index {block_index}"
            ))),
        }
    }

    pub(super) fn append_thinking_delta(
        &mut self,
        block_index: u32,
        delta: &str,
    ) -> StreamTranslationResult<String> {
        match self.blocks.get_mut(&block_index) {
            Some(StreamBlock::Thinking { item_id, text }) => {
                text.push_str(delta);
                Ok(item_id.clone())
            }
            Some(
                StreamBlock::Text { .. }
                | StreamBlock::RedactedThinking { .. }
                | StreamBlock::ToolUse { .. },
            ) => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted thinking_delta for incompatible content block index {block_index}"
            ))),
            None => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted thinking_delta for unopened content block index {block_index}"
            ))),
        }
    }

    pub(super) fn append_tool_arguments_delta(
        &mut self,
        block_index: u32,
        delta: &str,
    ) -> StreamTranslationResult<String> {
        match self.blocks.get_mut(&block_index) {
            Some(StreamBlock::ToolUse {
                item_id, arguments, ..
            }) => {
                arguments.push_str(delta);
                Ok(item_id.clone())
            }
            Some(
                StreamBlock::Text { .. }
                | StreamBlock::Thinking { .. }
                | StreamBlock::RedactedThinking { .. },
            ) => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted input_json_delta for incompatible content block index {block_index}"
            ))),
            None => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted input_json_delta for unopened content block index {block_index}"
            ))),
        }
    }

    pub(super) fn require_reasoning_signature_block(
        &self,
        block_index: u32,
    ) -> StreamTranslationResult<()> {
        let Some(actual) = self.blocks.get(&block_index) else {
            return Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted signature_delta for unopened content block index {block_index}"
            )));
        };
        if !matches!(actual, StreamBlock::Thinking { .. }) {
            return Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted signature_delta for incompatible content block index {block_index}"
            )));
        }
        Ok(())
    }

    pub(super) fn stop_block(&mut self, block_index: u32) -> StreamTranslationResult<StreamBlock> {
        self.blocks.remove(&block_index).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted content_block_stop for unopened content block index {block_index}"
            ))
        })
    }
}
