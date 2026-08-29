//! Per-message projection state for
//! `openai_responses -> openai_chat_completions` streaming translation.
//!
//! Responses source lifecycle (response identity and output-item ordering) is
//! owned by `ResponsesInboundLifecycle`. This state owns only the compact Chat
//! tool-call indexes and source-content prefixes needed to reconcile Responses
//! final snapshots without duplicating already forwarded deltas.

use std::collections::BTreeMap;

use crate::translation::openai_responses::streaming::{
    ForwardedContent, ResponsesOutputSegmentKey,
};
use crate::translation::stream::{StreamTranslationError, StreamTranslationResult};

/// In-flight Chat projection state for one Responses assistant turn.
#[derive(Debug, Default)]
pub(super) struct StreamingState {
    message_started: bool,
    next_tool_call_index: u32,
    tool_call_indexes: BTreeMap<u32, u32>,
    forwarded_content: BTreeMap<ResponsesOutputSegmentKey, ForwardedContent>,
}

impl StreamingState {
    /// Record that the assistant role delta has been emitted. Returns true if
    /// this call is the one that flipped the flag (i.e. the caller should emit
    /// the role delta chunk).
    pub(super) fn mark_message_started(&mut self) -> bool {
        if self.message_started {
            false
        } else {
            self.message_started = true;
            true
        }
    }

    /// Allocate the compact Chat tool-call index for a Responses output item.
    ///
    /// `ResponsesInboundLifecycle` has already rejected duplicate
    /// `response.output_item.added` events before this pair state is reached.
    pub(super) fn register_tool_call(&mut self, output_index: u32) -> u32 {
        let tool_call_index = self.next_tool_call_index;
        self.next_tool_call_index += 1;
        self.tool_call_indexes.insert(output_index, tool_call_index);
        tool_call_index
    }

    pub(super) fn require_tool_call_index(
        &self,
        output_index: u32,
    ) -> StreamTranslationResult<u32> {
        self.tool_call_indexes.get(&output_index).copied().ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Responses stream emitted function arguments for output_index {output_index} before function_call output_item.added"
            ))
        })
    }

    pub(super) fn append_content(&mut self, key: ResponsesOutputSegmentKey, delta: &str) {
        self.forwarded_content.entry(key).or_default().append(delta);
    }

    /// Reconcile a Responses `*.done` or enclosing `*_part.done` snapshot with
    /// previously forwarded deltas, returning only an unforwarded suffix.
    pub(super) fn reconcile_content_snapshot(
        &mut self,
        key: ResponsesOutputSegmentKey,
        final_content: &str,
        event: &str,
    ) -> StreamTranslationResult<Option<String>> {
        self.forwarded_content
            .entry(key)
            .or_default()
            .reconcile_snapshot(final_content)
            .map_err(|_| {
                StreamTranslationError::Semantic(format!(
                    "Responses stream emitted {event} whose final content diverged from streamed content for {}",
                    key.context()
                ))
            })
    }
}
