//! Per-message accumulation state for
//! `openai_responses -> openai_chat_completions` streaming translation.
//!
//! Responses stream events already carry their own lifecycle and output
//! indexes, so the only pair-private state is whether the assistant role delta
//! has been emitted and which tool-call output indexes have already introduced
//! a Chat `tool_calls` start chunk.

use std::collections::BTreeSet;

/// In-flight streaming state for a single Responses assistant turn projected
/// onto a Chat Completions stream.
#[derive(Debug, Default)]
pub(super) struct StreamingState {
    message_started: bool,
    tool_call_indexes: BTreeSet<u32>,
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

    /// Register a tool-call output index so later arguments deltas know not to
    /// repeat the id/name. Returns true if this is a new index.
    pub(super) fn register_tool_call(&mut self, output_index: u32) -> bool {
        self.tool_call_indexes.insert(output_index)
    }

    pub(super) fn has_tool_calls(&self) -> bool {
        !self.tool_call_indexes.is_empty()
    }
}
