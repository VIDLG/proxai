//! Per-message accumulation state for
//! `openai_chat_completions -> anthropic_messages` streaming translation.
//!
//! Tracks Chat Completions choice index, currently open Anthropic content
//! blocks (text + tool use), and accumulated refusal text so deltas can be
//! routed and finalized correctly. Holds no Anthropic event builders
//! directly; those live in `super::output`.

use std::collections::BTreeMap;

use crate::protocol::anthropic::messages::MessageStreamEvent;
use crate::protocol::openai::chat_completions::{
    ChatCompletionMessageToolCallChunk, CompletionUsage, FinishReason,
};

use crate::translation::anthropic_messages::outbound::{
    content_block_stop, input_json_delta, text_block_start, text_delta, thinking_block_start,
    thinking_delta, tool_use_block_start,
};
use crate::translation::stream::{StreamTranslationError, StreamTranslationResult};

#[derive(Debug, Default)]
pub(super) struct ChatToAnthropicBlockState {
    next_block_index: u32,
    text_block_index: Option<u32>,
    reasoning_block_index: Option<u32>,
    tool_block_indexes: BTreeMap<u32, u32>,
}

#[derive(Debug)]
pub(super) struct ChatStreamingState {
    pub(super) blocks: ChatToAnthropicBlockState,
    pub(super) refusal: String,
    choice_index: Option<u32>,
}

impl ChatStreamingState {
    pub(super) fn new() -> Self {
        Self {
            blocks: ChatToAnthropicBlockState::default(),
            refusal: String::new(),
            choice_index: None,
        }
    }

    pub(super) fn text_delta(&mut self, text: String) -> Vec<MessageStreamEvent> {
        match self.blocks.text_block_index {
            Some(index) => vec![text_delta(index, text)],
            None => {
                let index = self.blocks.allocate_block_index();
                self.blocks.text_block_index = Some(index);
                vec![text_block_start(index, text)]
            }
        }
    }

    pub(super) fn refusal_delta(&mut self, refusal: String) -> Vec<MessageStreamEvent> {
        self.refusal.push_str(&refusal);
        self.text_delta(refusal)
    }

    pub(super) fn reasoning_delta(&mut self, reasoning: String) -> Vec<MessageStreamEvent> {
        match self.blocks.reasoning_block_index {
            Some(index) => vec![thinking_delta(index, reasoning)],
            None => {
                let index = self.blocks.allocate_block_index();
                self.blocks.reasoning_block_index = Some(index);
                vec![
                    thinking_block_start(index),
                    thinking_delta(index, reasoning),
                ]
            }
        }
    }

    /// Whether any refusal text has been accumulated so far.
    pub(super) fn has_refusal(&self) -> bool {
        !self.refusal.is_empty()
    }

    /// Take ownership of the accumulated refusal text, leaving the state empty.
    pub(super) fn take_refusal(&mut self) -> String {
        std::mem::take(&mut self.refusal)
    }

    pub(super) fn tool_call_delta(
        &mut self,
        tool_call: ChatCompletionMessageToolCallChunk,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let arguments = tool_call
            .function
            .as_ref()
            .and_then(|function| function.arguments.clone())
            .filter(|arguments| !arguments.is_empty());

        let mut outputs = Vec::new();
        let block_index = match self
            .blocks
            .tool_block_indexes
            .get(&tool_call.index)
            .copied()
        {
            Some(index) => index,
            None => {
                let id = tool_call.id.clone().ok_or_else(|| {
                    StreamTranslationError::Semantic(
                        "Chat tool call stream started without a tool call id".to_string(),
                    )
                })?;
                let name = tool_call
                    .function
                    .as_ref()
                    .and_then(|function| function.name.clone())
                    .filter(|name| !name.is_empty())
                    .ok_or_else(|| {
                        StreamTranslationError::Semantic(
                            "Chat tool call stream started without a function name".to_string(),
                        )
                    })?;
                let index = self.blocks.allocate_block_index();
                self.blocks
                    .tool_block_indexes
                    .insert(tool_call.index, index);
                outputs.push(tool_use_block_start(index, id, name));
                index
            }
        };

        if let Some(arguments) = arguments {
            outputs.push(input_json_delta(block_index, arguments));
        }

        Ok(outputs)
    }

    pub(super) fn register_choice_index(&mut self, index: u32) -> StreamTranslationResult<()> {
        match self.choice_index {
            Some(existing) if existing != index => Err(StreamTranslationError::Semantic(format!(
                "Chat stream switched from choice index {existing} to {index}; Anthropic message streams can represent exactly one assistant message"
            ))),
            Some(_) => Ok(()),
            None => {
                self.choice_index = Some(index);
                Ok(())
            }
        }
    }
}

impl ChatToAnthropicBlockState {
    fn allocate_block_index(&mut self) -> u32 {
        let index = self.next_block_index;
        self.next_block_index = self.next_block_index.saturating_add(1);
        index
    }

    pub(super) fn stop_open_blocks(&mut self) -> Vec<MessageStreamEvent> {
        let mut indexes = Vec::new();
        if let Some(index) = self.text_block_index.take() {
            indexes.push(index);
        }
        if let Some(index) = self.reasoning_block_index.take() {
            indexes.push(index);
        }
        indexes.extend(self.tool_block_indexes.values().copied());
        self.tool_block_indexes.clear();
        indexes.sort_unstable();

        indexes.into_iter().map(content_block_stop).collect()
    }
}

#[derive(Debug)]
pub(super) struct PendingAnthropicTerminal {
    pub(super) finish_reason: FinishReason,
    pub(super) refusal: String,
    pub(super) usage: Option<CompletionUsage>,
}
