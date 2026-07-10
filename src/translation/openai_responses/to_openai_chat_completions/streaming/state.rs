//! Per-message accumulation state for
//! `openai_responses -> openai_chat_completions` streaming translation.
//!
//! Responses stream events already carry their own lifecycle and output
//! indexes, so the only pair-private state is whether the assistant role delta
//! has been emitted and which tool-call output indexes have already introduced
//! a Chat `tool_calls` start chunk.

use std::collections::BTreeMap;

use crate::translation::streaming::{StreamTranslationError, StreamTranslationResult};

/// In-flight streaming state for a single Responses assistant turn projected
/// onto a Chat Completions stream.
#[derive(Debug, Default)]
pub(super) struct StreamingState {
    message_started: bool,
    next_tool_call_index: u32,
    tool_call_indexes: BTreeMap<u32, u32>,
    emitted_text: bool,
    emitted_refusal: bool,
    emitted_reasoning: bool,
}

impl StreamingState {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Record that the assistant role delta has been emitted. Returns true if
    /// this call is the one that flipped the flag (i.e. the caller should emit
    /// the role delta chunk).
    pub(super) fn start_message(&mut self) -> bool {
        if self.message_started {
            false
        } else {
            self.message_started = true;
            true
        }
    }

    /// Register a Responses output index and allocate the compact Chat tool-call
    /// index used by subsequent argument deltas.
    pub(super) fn register_tool_call(&mut self, output_index: u32) -> Option<u32> {
        if self.tool_call_indexes.contains_key(&output_index) {
            return None;
        }
        let tool_call_index = self.next_tool_call_index;
        self.next_tool_call_index = self.next_tool_call_index.saturating_add(1);
        self.tool_call_indexes.insert(output_index, tool_call_index);
        Some(tool_call_index)
    }

    pub(super) fn tool_call_index(&self, output_index: u32) -> StreamTranslationResult<u32> {
        self.tool_call_indexes.get(&output_index).copied().ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Responses stream emitted function arguments for output_index {output_index} before function_call output_item.added"
            ))
        })
    }

    pub(super) fn mark_text(&mut self) {
        self.emitted_text = true;
    }

    pub(super) fn mark_refusal(&mut self) {
        self.emitted_refusal = true;
    }

    pub(super) fn mark_reasoning(&mut self) {
        self.emitted_reasoning = true;
    }

    pub(super) fn emitted_any(&self) -> bool {
        self.emitted_text
            || self.emitted_refusal
            || self.emitted_reasoning
            || !self.tool_call_indexes.is_empty()
    }

    pub(super) fn has_tool_calls(&self) -> bool {
        !self.tool_call_indexes.is_empty()
    }
}
