//! Per-block accumulation state for
//! `anthropic_messages -> openai_chat_completions` streaming translation.
//!
//! See `super::mod` for the translator that drives this state.

use std::collections::BTreeMap;

use crate::translation::anthropic_messages::continuation::{Continuation, ContinuationEnvelope};
use crate::translation::streaming::{StreamTranslationError, StreamTranslationResult};

/// In-flight streaming state for a single Anthropic assistant message.
///
/// Tracks content block registrations so delta/stop events can be validated
/// against the block variant they reference. Holds no protocol output
/// directly; output building lives in `super::output`.
#[derive(Debug)]
pub(super) struct StreamingState {
    blocks: BTreeMap<u32, StreamBlock>,
    continuation: Option<ContinuationEnvelope>,
    next_tool_call_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum StreamBlock {
    Text,
    ToolUse { chat_tool_index: u32 },
    Thinking { text: String, signature: String },
    RedactedThinking { data: String },
}

impl StreamingState {
    pub(super) fn new() -> Self {
        Self {
            blocks: BTreeMap::new(),
            continuation: None,
            next_tool_call_index: 0,
        }
    }

    pub(super) fn register_tool_use_block(
        &mut self,
        block_index: u32,
    ) -> StreamTranslationResult<u32> {
        let tool_call_index = self.next_tool_call_index();
        self.register_block(
            block_index,
            StreamBlock::ToolUse {
                chat_tool_index: tool_call_index,
            },
        )?;
        Ok(tool_call_index)
    }

    fn next_tool_call_index(&mut self) -> u32 {
        let index = self.next_tool_call_index;
        self.next_tool_call_index = self.next_tool_call_index.saturating_add(1);
        index
    }

    pub(super) fn register_text_block(&mut self, block_index: u32) -> StreamTranslationResult<()> {
        self.register_block(block_index, StreamBlock::Text)
    }

    pub(super) fn register_thinking_block(
        &mut self,
        block_index: u32,
        text: String,
        signature: String,
    ) -> StreamTranslationResult<()> {
        self.register_block(block_index, StreamBlock::Thinking { text, signature })
    }

    pub(super) fn register_redacted_thinking_block(
        &mut self,
        block_index: u32,
        data: String,
    ) -> StreamTranslationResult<()> {
        self.register_block(block_index, StreamBlock::RedactedThinking { data })
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

    fn opened_block(
        &self,
        block_index: u32,
        event_name: &'static str,
    ) -> StreamTranslationResult<&StreamBlock> {
        self.blocks.get(&block_index).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted {event_name} for unopened content block index {block_index}"
            ))
        })
    }

    fn opened_block_mut(
        &mut self,
        block_index: u32,
        event_name: &'static str,
    ) -> StreamTranslationResult<&mut StreamBlock> {
        self.blocks.get_mut(&block_index).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted {event_name} for unopened content block index {block_index}"
            ))
        })
    }

    pub(super) fn require_text_block(
        &self,
        block_index: u32,
        delta_name: &'static str,
    ) -> StreamTranslationResult<()> {
        match self.opened_block(block_index, delta_name)? {
            StreamBlock::Text => Ok(()),
            _ => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted {delta_name} for incompatible content block index {block_index}"
            ))),
        }
    }

    pub(super) fn append_thinking_delta(
        &mut self,
        block_index: u32,
        delta: &str,
    ) -> StreamTranslationResult<()> {
        let StreamBlock::Thinking { text, .. } =
            self.opened_block_mut(block_index, "thinking_delta")?
        else {
            return Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted thinking_delta for incompatible content block index {block_index}"
            )));
        };
        text.push_str(delta);
        Ok(())
    }

    pub(super) fn append_signature_delta(
        &mut self,
        block_index: u32,
        delta: &str,
    ) -> StreamTranslationResult<()> {
        let StreamBlock::Thinking { signature, .. } =
            self.opened_block_mut(block_index, "signature_delta")?
        else {
            return Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted signature_delta for incompatible content block index {block_index}"
            )));
        };
        signature.push_str(delta);
        Ok(())
    }

    pub(super) fn get_tool_call_index(&self, block_index: u32) -> StreamTranslationResult<u32> {
        let StreamBlock::ToolUse { chat_tool_index } =
            self.opened_block(block_index, "input_json_delta")?
        else {
            return Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted input_json_delta for incompatible content block index {block_index}"
            )));
        };
        Ok(*chat_tool_index)
    }

    pub(super) fn stop_block(&mut self, block_index: u32) -> StreamTranslationResult<bool> {
        let block = self.blocks.remove(&block_index).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted content_block_stop for unopened content block index {block_index}"
            ))
        })?;
        let produced_continuation = match block {
            StreamBlock::Thinking { text, signature } => {
                self.continuation
                    .get_or_insert_default()
                    .push(Continuation::Thinking {
                        thinking: text,
                        signature,
                    });
                true
            }
            StreamBlock::RedactedThinking { data } => {
                self.continuation
                    .get_or_insert_default()
                    .push(Continuation::RedactedThinking { data });
                true
            }
            StreamBlock::Text | StreamBlock::ToolUse { .. } => false,
        };
        Ok(produced_continuation)
    }

    pub(super) fn take_continuation(&mut self) -> Option<ContinuationEnvelope> {
        self.continuation.take()
    }
}
