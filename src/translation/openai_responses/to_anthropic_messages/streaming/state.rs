//! Target-side state for
//! `openai_responses -> anthropic_messages` streaming translation.
//!
//! Envelope-level state (response identity, stream lifecycle phase) is owned
//! by
//! [`ResponsesInboundLifecycle`](crate::translation::openai_responses::streaming::ResponsesInboundLifecycle).
//! This module tracks only the Anthropic-message-specific projection:
//! whether `message_start` has been emitted, currently open content blocks,
//! the latest usage snapshot (for `message_delta`), and whether the stream
//! has emitted its terminal `message_stop`.

use std::collections::{BTreeMap, BTreeSet};

use crate::protocol::anthropic::messages::{MessageStreamEvent, StopReason};
use crate::protocol::openai_responses::OutputItem;

use super::output;

/// Per-stream Anthropic projection state.
///
/// Identity (message id + model) lives on the lifecycle's `StreamIdentity`;
/// this struct owns only Anthropic-specific projection state. The latest
/// usage snapshot is kept here because `message_delta` (the Anthropic
/// terminal envelope) needs `input_tokens` / `output_tokens`, and the
/// generic lifecycle does not model usage.
#[derive(Debug, Default)]
pub(super) struct StreamingState {
    message_started: bool,
    input_tokens: u32,
    output_tokens: u32,
    blocks: BTreeMap<u32, StreamBlockKind>,
    stopped_blocks: BTreeSet<u32>,
    completed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamBlockKind {
    Text,
    Thinking,
    ToolUse,
}

impl StreamingState {
    /// Capture the latest usage snapshot from a Responses envelope so the
    /// terminal `message_delta` carries correct `input_tokens` /
    /// `output_tokens`. Identity is intentionally not stored here; it lives
    /// on the lifecycle and is read at event-construction time.
    pub(super) fn record_usage(&mut self, response: &crate::protocol::openai_responses::Response) {
        if let Some(usage) = response.usage {
            self.input_tokens = usage.input_tokens;
            self.output_tokens = usage.output_tokens;
        }
    }

    /// Emit `message_start` on the first call.
    pub(super) fn start_message(
        &mut self,
        identity: &crate::translation::streaming::StreamIdentity,
    ) -> Option<MessageStreamEvent> {
        self.start_message_event(identity)
    }

    /// Emit `message_start` if it has not been emitted yet, returning an
    /// empty Vec otherwise. Used by content-event handlers so the Anthropic
    /// stream stays well-formed even if a content delta arrived before any
    /// `response.created` / `response.in_progress` event.
    pub(super) fn ensure_message_started(
        &mut self,
        identity: &crate::translation::streaming::StreamIdentity,
    ) -> Vec<MessageStreamEvent> {
        self.start_message_event(identity).into_iter().collect()
    }

    pub(super) fn register_output_item(
        &mut self,
        output_index: u32,
        item: OutputItem,
    ) -> Option<MessageStreamEvent> {
        if self.blocks.contains_key(&output_index) {
            return None;
        }

        match item {
            OutputItem::Message(_) => self.open_text_block(output_index),
            OutputItem::Reasoning(_) => self.open_thinking_block(output_index),
            OutputItem::FunctionCall(item) => {
                self.open_tool_block(output_index, Some(item.call_id), Some(item.name))
            }
            OutputItem::CustomToolCall(item) => {
                self.open_tool_block(output_index, Some(item.call_id), Some(item.name))
            }
            other => {
                tracing::trace!(
                    output_item_type = output_item_type(&other),
                    reason = "Responses output item has no Anthropic Messages stream block representation"
                );
                None
            }
        }
    }

    pub(super) fn ensure_text_block(&mut self, index: u32) -> Option<MessageStreamEvent> {
        if self.blocks.contains_key(&index) {
            return None;
        }
        self.open_text_block(index)
    }

    pub(super) fn ensure_thinking_block(&mut self, index: u32) -> Option<MessageStreamEvent> {
        if self.blocks.contains_key(&index) {
            return None;
        }
        self.open_thinking_block(index)
    }

    pub(super) fn ensure_tool_block(
        &mut self,
        index: u32,
        item_id: Option<String>,
        name: Option<String>,
    ) -> Option<MessageStreamEvent> {
        if self.blocks.contains_key(&index) {
            return None;
        }
        self.open_tool_block(index, item_id, name)
    }

    pub(super) fn stop_block(&mut self, index: u32) -> Option<MessageStreamEvent> {
        if self.stopped_blocks.insert(index) {
            return Some(output::content_block_stop(index));
        }
        None
    }

    pub(super) fn complete(&mut self, stop_reason: StopReason) -> Vec<MessageStreamEvent> {
        if self.completed {
            return Vec::new();
        }
        self.completed = true;

        let mut events = Vec::new();
        let open_indexes = self.blocks.keys().copied().collect::<Vec<_>>();
        for index in open_indexes {
            if let Some(event) = self.stop_block(index) {
                events.push(event);
            }
        }
        events.push(output::message_delta(
            stop_reason,
            self.input_tokens,
            self.output_tokens,
        ));
        events.push(output::message_stop());
        events
    }

    fn start_message_event(
        &mut self,
        identity: &crate::translation::streaming::StreamIdentity,
    ) -> Option<MessageStreamEvent> {
        if self.message_started {
            return None;
        }
        self.message_started = true;
        Some(output::message_start(
            identity.id().to_string(),
            identity.model().to_string(),
            self.input_tokens,
        ))
    }

    fn open_text_block(&mut self, index: u32) -> Option<MessageStreamEvent> {
        self.blocks.insert(index, StreamBlockKind::Text);
        Some(output::text_block_start(index))
    }

    fn open_thinking_block(&mut self, index: u32) -> Option<MessageStreamEvent> {
        self.blocks.insert(index, StreamBlockKind::Thinking);
        Some(output::thinking_block_start(index))
    }

    fn open_tool_block(
        &mut self,
        index: u32,
        item_id: Option<String>,
        name: Option<String>,
    ) -> Option<MessageStreamEvent> {
        let id = item_id.unwrap_or_else(|| format!("toolu_{index}"));
        let name = name.unwrap_or_else(|| "function".to_string());
        self.blocks.insert(index, StreamBlockKind::ToolUse);
        Some(output::tool_use_block_start(index, id, name))
    }
}

fn output_item_type(item: &OutputItem) -> &'static str {
    match item {
        OutputItem::Message(_) => "message",
        OutputItem::FileSearchCall(_) => "file_search_call",
        OutputItem::FunctionCall(_) => "function_call",
        OutputItem::FunctionCallOutput(_) => "function_call_output",
        OutputItem::WebSearchCall(_) => "web_search_call",
        OutputItem::ComputerCall(_) => "computer_call",
        OutputItem::ComputerCallOutput(_) => "computer_call_output",
        OutputItem::Reasoning(_) => "reasoning",
        OutputItem::Compaction(_) => "compaction",
        OutputItem::ImageGenerationCall(_) => "image_generation_call",
        OutputItem::CodeInterpreterCall(_) => "code_interpreter_call",
        OutputItem::LocalShellCall(_) => "local_shell_call",
        OutputItem::ShellCall(_) => "shell_call",
        OutputItem::ShellCallOutput(_) => "shell_call_output",
        OutputItem::ApplyPatchCall(_) => "apply_patch_call",
        OutputItem::ApplyPatchCallOutput(_) => "apply_patch_call_output",
        OutputItem::McpCall(_) => "mcp_call",
        OutputItem::McpListTools(_) => "mcp_list_tools",
        OutputItem::McpApprovalRequest(_) => "mcp_approval_request",
        OutputItem::CustomToolCall(_) => "custom_tool_call",
        OutputItem::CustomToolCallOutput(_) => "custom_tool_call_output",
        OutputItem::ToolSearchCall(_) => "tool_search_call",
        OutputItem::ToolSearchOutput(_) => "tool_search_output",
    }
}
