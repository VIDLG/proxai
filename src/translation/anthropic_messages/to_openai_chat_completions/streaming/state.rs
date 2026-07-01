//! Per-block accumulation state for
//! `anthropic_messages -> openai_chat_completions` streaming translation.
//!
//! See `super::mod` for the translator that drives this state.

use std::collections::BTreeMap;

use crate::translation::streaming::{StreamTranslationError, StreamTranslationResult};

/// In-flight streaming state for a single Anthropic assistant message.
///
/// Tracks content block registrations so delta/stop events can be validated
/// against the block variant they reference. Holds no protocol output
/// directly; output building lives in `super::output`.
#[derive(Debug)]
pub(super) struct StreamingState {
    blocks: BTreeMap<u32, StreamBlock>,
    next_tool_call_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StreamBlock {
    Text,
    ToolUse { chat_tool_index: u32 },
    Thinking,
    Ignored,
}

impl StreamingState {
    pub(super) fn new() -> Self {
        Self {
            blocks: BTreeMap::new(),
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
    ) -> StreamTranslationResult<()> {
        self.register_block(block_index, StreamBlock::Thinking)
    }

    pub(super) fn register_ignored_block(
        &mut self,
        block_index: u32,
    ) -> StreamTranslationResult<()> {
        self.register_block(block_index, StreamBlock::Ignored)
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

    pub(super) fn require_block(
        &self,
        block_index: u32,
        expected: StreamBlock,
        delta_name: &'static str,
    ) -> StreamTranslationResult<()> {
        let Some(actual) = self.blocks.get(&block_index).copied() else {
            return Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted {delta_name} for unopened content block index {block_index}"
            )));
        };
        if actual != expected {
            return Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted {delta_name} for incompatible content block index {block_index}"
            )));
        }
        Ok(())
    }

    pub(super) fn require_reasoning_signature_block(
        &self,
        block_index: u32,
    ) -> StreamTranslationResult<()> {
        let Some(actual) = self.blocks.get(&block_index).copied() else {
            return Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted signature_delta for unopened content block index {block_index}"
            )));
        };
        if !matches!(actual, StreamBlock::Thinking | StreamBlock::Ignored) {
            return Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted signature_delta for incompatible content block index {block_index}"
            )));
        }
        Ok(())
    }

    pub(super) fn get_tool_call_index(&self, block_index: u32) -> StreamTranslationResult<u32> {
        match self.blocks.get(&block_index).copied() {
            Some(StreamBlock::ToolUse { chat_tool_index }) => Ok(chat_tool_index),
            Some(StreamBlock::Text | StreamBlock::Thinking | StreamBlock::Ignored) => {
                Err(StreamTranslationError::Semantic(format!(
                    "Anthropic stream emitted input_json_delta for incompatible content block index {block_index}"
                )))
            }
            None => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted input_json_delta for unopened content block index {block_index}"
            ))),
        }
    }

    pub(super) fn stop_block(&mut self, block_index: u32) -> StreamTranslationResult<()> {
        self.blocks.remove(&block_index).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted content_block_stop for unopened content block index {block_index}"
            ))
        })?;
        Ok(())
    }
}
