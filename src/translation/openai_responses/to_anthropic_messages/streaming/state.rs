//! Target-side state for
//! `openai_responses -> anthropic_messages` streaming translation.
//!
//! Envelope-level state (response identity, stream lifecycle phase) is owned
//! by
//! [`ResponsesInboundLifecycle`](crate::translation::openai_responses::streaming::ResponsesInboundLifecycle).
//! This module tracks only the Anthropic-message-specific projection:
//! whether `message_start` has been emitted, currently open content blocks,
//! and whether the stream has emitted its terminal `message_stop`.

use std::collections::{BTreeMap, BTreeSet};

use strum::Display;

use crate::protocol::anthropic::messages::{MessageDelta, MessageStreamEvent, StopReason, Usage};
use crate::protocol::openai_responses::{OutputContent, OutputItem, ResponseUsage, SummaryPart};

use crate::translation::anthropic_messages::outbound::{
    content_block_stop, input_json_delta as input_json_delta_event,
    message_delta as message_delta_event, message_start as message_start_event, message_stop,
    text_block_start, text_delta as text_delta_event, thinking_block_start,
    thinking_delta as thinking_delta_event, tool_use_block_start,
};
use crate::translation::streaming::{
    StreamIdentity, StreamTranslationError, StreamTranslationResult,
};

/// Per-stream Anthropic projection state.
///
/// Identity (message id + model) lives on the lifecycle's `StreamIdentity`;
/// this struct owns only Anthropic-specific projection state.
#[derive(Debug, Default)]
pub(super) struct StreamingState {
    message_started: bool,
    next_block_index: u32,
    registered_outputs: BTreeMap<u32, RegisteredOutputKind>,
    completed_outputs: BTreeSet<u32>,
    active_blocks: BTreeMap<StreamBlockKey, ActiveBlock>,
    stopped_blocks: BTreeSet<StreamBlockKey>,
    custom_tool_inputs: BTreeMap<u32, String>,
    saw_refusal: bool,
    completed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
enum RegisteredOutputKind {
    Message,
    Reasoning,
    FunctionToolUse,
    CustomToolUse,
    Ignored,
}

#[derive(Debug)]
struct ActiveBlock {
    index: u32,
    forwarded: String,
}

/// Source-side identity for a Responses output sub-block that has been mapped
/// to an Anthropic `content_block.index`.
///
/// This is intentionally only a key, not a complete content accumulator. The
/// small `forwarded` prefix stored in `ActiveBlock` is used only to reconcile
/// authoritative Responses `*.done` snapshots without duplicating deltas. The
/// reverse pair (`anthropic_messages -> openai_responses`) needs a heavier
/// `StreamBlock` enum because it must assemble finalized Responses `OutputItem`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum StreamBlockKey {
    Text {
        output_index: u32,
        content_index: u32,
    },
    Refusal {
        output_index: u32,
        content_index: u32,
    },
    Thinking {
        output_index: u32,
        content_index: u32,
    },
    Summary {
        output_index: u32,
        summary_index: u32,
    },
    ToolUse {
        output_index: u32,
    },
}

impl StreamingState {
    /// Emit `message_start` on the first call.
    pub(super) fn start_message(
        &mut self,
        identity: &StreamIdentity,
        usage: Option<&ResponseUsage>,
    ) -> Option<MessageStreamEvent> {
        if self.message_started {
            return None;
        }
        self.message_started = true;
        Some(message_start_event(
            identity.id().to_string(),
            identity.model().to_string(),
            usage.map(Usage::from).unwrap_or_default(),
        ))
    }

    pub(super) fn register_output_item(
        &mut self,
        output_index: u32,
        item: OutputItem,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        if self.registered_outputs.contains_key(&output_index)
            || self.completed_outputs.contains(&output_index)
        {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted duplicate {event} for output_index {output_index}"
            )));
        }

        match item {
            OutputItem::Message(_) => {
                self.registered_outputs
                    .insert(output_index, RegisteredOutputKind::Message);
                Ok(Vec::new())
            }
            OutputItem::Reasoning(_) => {
                self.registered_outputs
                    .insert(output_index, RegisteredOutputKind::Reasoning);
                Ok(Vec::new())
            }
            OutputItem::FunctionCall(item) => {
                let key = StreamBlockKey::ToolUse { output_index };
                let start = self.open_tool_block(key, item.call_id, item.name)?;
                let mut events = vec![start];
                if !item.arguments.is_empty() {
                    let index = self.append_active_content(key, &item.arguments, event)?;
                    events.push(input_json_delta_event(index, item.arguments));
                }
                self.registered_outputs
                    .insert(output_index, RegisteredOutputKind::FunctionToolUse);
                Ok(events)
            }
            OutputItem::CustomToolCall(item) => {
                let key = StreamBlockKey::ToolUse { output_index };
                let start = self.open_tool_block(key, item.call_id, item.name)?;
                self.custom_tool_inputs.insert(output_index, item.input);
                self.registered_outputs
                    .insert(output_index, RegisteredOutputKind::CustomToolUse);
                Ok(vec![start])
            }
            other => {
                tracing::trace!(
                    item_type = other.as_ref(),
                    reason =
                        "Responses output item has no Anthropic Messages stream representation"
                );
                self.registered_outputs
                    .insert(output_index, RegisteredOutputKind::Ignored);
                Ok(Vec::new())
            }
        }
    }

    pub(super) fn finish_output_item(
        &mut self,
        output_index: u32,
        event: &str,
    ) -> StreamTranslationResult<()> {
        if !self.registered_outputs.contains_key(&output_index) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for output_index {output_index} before response.output_item.added"
            )));
        }
        if self.completed_outputs.contains(&output_index) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted duplicate {event} for output_index {output_index}"
            )));
        }
        if let Some((key, _)) = self
            .active_blocks
            .iter()
            .find(|(key, _)| block_output_index(**key) == output_index)
        {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} before {} was closed",
                block_key_context(*key)
            )));
        }
        self.completed_outputs.insert(output_index);
        Ok(())
    }

    pub(super) fn register_content_part(
        &mut self,
        output_index: u32,
        content_index: u32,
        part: OutputContent,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        match part {
            OutputContent::OutputText(part) => {
                self.require_output_kind(output_index, RegisteredOutputKind::Message, event)?;
                let key = StreamBlockKey::Text {
                    output_index,
                    content_index,
                };
                Ok(vec![self.open_text_block(key, part.text)?])
            }
            OutputContent::Refusal(part) => {
                self.require_output_kind(output_index, RegisteredOutputKind::Message, event)?;
                self.saw_refusal = true;
                let key = StreamBlockKey::Refusal {
                    output_index,
                    content_index,
                };
                Ok(vec![self.open_text_block(key, part.refusal)?])
            }
            OutputContent::ReasoningText(part) => {
                self.require_output_kind(output_index, RegisteredOutputKind::Reasoning, event)?;
                let key = StreamBlockKey::Thinking {
                    output_index,
                    content_index,
                };
                self.open_thinking_block(key, part.text, event)
            }
        }
    }

    pub(super) fn finish_content_part(
        &mut self,
        output_index: u32,
        content_index: u32,
        part: OutputContent,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let (key, final_content, expected) = match part {
            OutputContent::OutputText(part) => (
                StreamBlockKey::Text {
                    output_index,
                    content_index,
                },
                part.text,
                RegisteredOutputKind::Message,
            ),
            OutputContent::Refusal(part) => {
                self.saw_refusal = true;
                (
                    StreamBlockKey::Refusal {
                        output_index,
                        content_index,
                    },
                    part.refusal,
                    RegisteredOutputKind::Message,
                )
            }
            OutputContent::ReasoningText(part) => (
                StreamBlockKey::Thinking {
                    output_index,
                    content_index,
                },
                part.text,
                RegisteredOutputKind::Reasoning,
            ),
        };
        self.require_output_kind(output_index, expected, event)?;

        // OpenAI-compatible upstreams differ on whether `*.done` or
        // `content_part.done` closes the semantic part. If the protocol-specific
        // done event already closed it, this envelope event is only validation.
        if self.stopped_blocks.contains(&key) {
            return Ok(Vec::new());
        }
        self.finish_text_like_block(key, final_content, event)
    }

    pub(super) fn text_delta(
        &mut self,
        output_index: u32,
        content_index: u32,
        delta: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.require_output_kind(output_index, RegisteredOutputKind::Message, event)?;
        let key = StreamBlockKey::Text {
            output_index,
            content_index,
        };
        self.forward_text_like_delta(key, delta, event, false)
    }

    pub(super) fn finish_text(
        &mut self,
        output_index: u32,
        content_index: u32,
        text: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.require_output_kind(output_index, RegisteredOutputKind::Message, event)?;
        self.finish_text_like_block(
            StreamBlockKey::Text {
                output_index,
                content_index,
            },
            text,
            event,
        )
    }

    pub(super) fn refusal_delta(
        &mut self,
        output_index: u32,
        content_index: u32,
        delta: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.require_output_kind(output_index, RegisteredOutputKind::Message, event)?;
        self.saw_refusal = true;
        let key = StreamBlockKey::Refusal {
            output_index,
            content_index,
        };
        self.forward_text_like_delta(key, delta, event, false)
    }

    pub(super) fn finish_refusal(
        &mut self,
        output_index: u32,
        content_index: u32,
        refusal: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.require_output_kind(output_index, RegisteredOutputKind::Message, event)?;
        self.saw_refusal = true;
        self.finish_text_like_block(
            StreamBlockKey::Refusal {
                output_index,
                content_index,
            },
            refusal,
            event,
        )
    }

    pub(super) fn register_summary_part(
        &mut self,
        output_index: u32,
        summary_index: u32,
        part: SummaryPart,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.require_output_kind(output_index, RegisteredOutputKind::Reasoning, event)?;
        let SummaryPart::SummaryText(part) = part;
        self.open_thinking_block(
            StreamBlockKey::Summary {
                output_index,
                summary_index,
            },
            part.text,
            event,
        )
    }

    pub(super) fn summary_delta(
        &mut self,
        output_index: u32,
        summary_index: u32,
        delta: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.require_output_kind(output_index, RegisteredOutputKind::Reasoning, event)?;
        let key = StreamBlockKey::Summary {
            output_index,
            summary_index,
        };
        self.forward_text_like_delta(key, delta, event, true)
    }

    pub(super) fn finish_summary_text(
        &mut self,
        output_index: u32,
        summary_index: u32,
        text: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.require_output_kind(output_index, RegisteredOutputKind::Reasoning, event)?;
        let key = StreamBlockKey::Summary {
            output_index,
            summary_index,
        };
        self.reconcile_active_content(key, &text, event)
            .map(|suffix| {
                suffix
                    .map(|(index, suffix)| thinking_delta_event(index, suffix))
                    .into_iter()
                    .collect()
            })
    }

    pub(super) fn stop_summary_part(
        &mut self,
        output_index: u32,
        summary_index: u32,
        part: SummaryPart,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.require_output_kind(output_index, RegisteredOutputKind::Reasoning, event)?;
        let SummaryPart::SummaryText(part) = part;
        self.finish_text_like_block(
            StreamBlockKey::Summary {
                output_index,
                summary_index,
            },
            part.text,
            event,
        )
    }

    pub(super) fn reasoning_text_delta(
        &mut self,
        output_index: u32,
        content_index: u32,
        delta: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.require_output_kind(output_index, RegisteredOutputKind::Reasoning, event)?;
        let key = StreamBlockKey::Thinking {
            output_index,
            content_index,
        };
        self.forward_text_like_delta(key, delta, event, true)
    }

    pub(super) fn stop_reasoning_text(
        &mut self,
        output_index: u32,
        content_index: u32,
        text: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.require_output_kind(output_index, RegisteredOutputKind::Reasoning, event)?;
        self.finish_text_like_block(
            StreamBlockKey::Thinking {
                output_index,
                content_index,
            },
            text,
            event,
        )
    }

    pub(super) fn function_arguments_delta(
        &mut self,
        output_index: u32,
        delta: String,
        event: &str,
    ) -> StreamTranslationResult<MessageStreamEvent> {
        self.require_output_kind(output_index, RegisteredOutputKind::FunctionToolUse, event)?;
        let key = StreamBlockKey::ToolUse { output_index };
        let index = self.append_active_content(key, &delta, event)?;
        Ok(input_json_delta_event(index, delta))
    }

    pub(super) fn finish_function_arguments(
        &mut self,
        output_index: u32,
        arguments: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.require_output_kind(output_index, RegisteredOutputKind::FunctionToolUse, event)?;
        let key = StreamBlockKey::ToolUse { output_index };
        let mut events = Vec::new();
        if let Some((index, suffix)) = self.reconcile_active_content(key, &arguments, event)? {
            events.push(input_json_delta_event(index, suffix));
        }
        events.push(self.stop_block(key, event)?);
        Ok(events)
    }

    pub(super) fn custom_tool_input_delta(
        &mut self,
        output_index: u32,
        delta: &str,
        event: &str,
    ) -> StreamTranslationResult<()> {
        self.require_output_kind(output_index, RegisteredOutputKind::CustomToolUse, event)?;
        self.require_block(StreamBlockKey::ToolUse { output_index }, event)?;
        self.custom_tool_inputs
            .get_mut(&output_index)
            .expect("custom tool input is registered with its output")
            .push_str(delta);
        Ok(())
    }

    pub(super) fn finish_custom_tool_input(
        &mut self,
        output_index: u32,
        input: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.require_output_kind(output_index, RegisteredOutputKind::CustomToolUse, event)?;
        let observed = self.custom_tool_inputs.remove(&output_index).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} without registered custom tool input state for output_index {output_index}"
            ))
        })?;
        if !input.starts_with(&observed) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} whose final custom tool input diverged from streamed input for output_index {output_index}"
            )));
        }

        let key = StreamBlockKey::ToolUse { output_index };
        let encoded = serde_json::to_string(&input)?;
        let index = self.append_active_content(key, &encoded, event)?;
        Ok(vec![
            input_json_delta_event(index, encoded),
            self.stop_block(key, event)?,
        ])
    }

    pub(super) fn complete(
        &mut self,
        stop_reason: StopReason,
        usage: Option<&ResponseUsage>,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        if self.completed {
            return Ok(Vec::new());
        }
        if let Some((key, _)) = self.active_blocks.iter().next() {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream completed before {} was closed",
                block_key_context(*key)
            )));
        }
        self.completed = true;
        let stop_reason = if self.saw_refusal && matches!(stop_reason, StopReason::EndTurn) {
            StopReason::Refusal
        } else {
            stop_reason
        };

        Ok(vec![
            message_delta_event(
                MessageDelta {
                    container: None,
                    stop_details: None,
                    stop_reason: Some(stop_reason),
                    stop_sequence: None,
                },
                usage.map(Into::into).unwrap_or_default(),
            ),
            message_stop(),
        ])
    }

    fn forward_text_like_delta(
        &mut self,
        key: StreamBlockKey,
        delta: String,
        event: &str,
        thinking: bool,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let mut events = Vec::new();
        if !self.active_blocks.contains_key(&key) {
            events.extend(if thinking {
                self.open_thinking_block(key, String::new(), event)?
            } else {
                vec![self.open_text_block(key, String::new())?]
            });
        }
        let index = self.append_active_content(key, &delta, event)?;
        events.push(if thinking {
            thinking_delta_event(index, delta)
        } else {
            text_delta_event(index, delta)
        });
        Ok(events)
    }

    fn finish_text_like_block(
        &mut self,
        key: StreamBlockKey,
        final_content: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let thinking = matches!(
            key,
            StreamBlockKey::Thinking { .. } | StreamBlockKey::Summary { .. }
        );
        let mut events = Vec::new();
        if !self.active_blocks.contains_key(&key) {
            events.extend(if thinking {
                self.open_thinking_block(key, String::new(), event)?
            } else {
                vec![self.open_text_block(key, String::new())?]
            });
        }
        if let Some((index, suffix)) = self.reconcile_active_content(key, &final_content, event)? {
            events.push(if thinking {
                thinking_delta_event(index, suffix)
            } else {
                text_delta_event(index, suffix)
            });
        }
        events.push(self.stop_block(key, event)?);
        Ok(events)
    }

    fn open_text_block(
        &mut self,
        key: StreamBlockKey,
        initial: String,
    ) -> StreamTranslationResult<MessageStreamEvent> {
        let index = self.open_block_index(key, initial.clone())?;
        Ok(text_block_start(index, initial))
    }

    fn open_thinking_block(
        &mut self,
        key: StreamBlockKey,
        initial: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let index = self.open_block_index(key, String::new())?;
        let mut events = vec![thinking_block_start(index)];
        if !initial.is_empty() {
            self.append_active_content(key, &initial, event)?;
            events.push(thinking_delta_event(index, initial));
        }
        Ok(events)
    }

    fn open_tool_block(
        &mut self,
        key: StreamBlockKey,
        item_id: String,
        name: String,
    ) -> StreamTranslationResult<MessageStreamEvent> {
        let index = self.open_block_index(key, String::new())?;
        Ok(tool_use_block_start(index, item_id, name))
    }

    fn open_block_index(
        &mut self,
        key: StreamBlockKey,
        forwarded: String,
    ) -> StreamTranslationResult<u32> {
        if self.active_blocks.contains_key(&key) || self.stopped_blocks.contains(&key) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream attempted to open duplicate Anthropic {} content block for {}",
                block_kind_name(key),
                block_key_context(key)
            )));
        }
        let index = self.next_block_index;
        self.next_block_index += 1;
        self.active_blocks
            .insert(key, ActiveBlock { index, forwarded });
        Ok(index)
    }

    fn append_active_content(
        &mut self,
        key: StreamBlockKey,
        delta: &str,
        event: &str,
    ) -> StreamTranslationResult<u32> {
        if self.stopped_blocks.contains(&key) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for {} after the Anthropic content block was already closed",
                block_key_context(key)
            )));
        }
        let block = self.active_blocks.get_mut(&key).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for {} before the corresponding Responses start event",
                block_key_context(key)
            ))
        })?;
        block.forwarded.push_str(delta);
        Ok(block.index)
    }

    fn reconcile_active_content(
        &mut self,
        key: StreamBlockKey,
        final_content: &str,
        event: &str,
    ) -> StreamTranslationResult<Option<(u32, String)>> {
        let block = self.active_blocks.get_mut(&key).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for {} before the corresponding Responses start event",
                block_key_context(key)
            ))
        })?;
        if !final_content.starts_with(&block.forwarded) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} whose final content diverged from streamed content for {}",
                block_key_context(key)
            )));
        }
        let suffix = final_content[block.forwarded.len()..].to_string();
        if suffix.is_empty() {
            return Ok(None);
        }
        block.forwarded.push_str(&suffix);
        Ok(Some((block.index, suffix)))
    }

    fn stop_block(
        &mut self,
        key: StreamBlockKey,
        event: &str,
    ) -> StreamTranslationResult<MessageStreamEvent> {
        let index = self.require_block(key, event)?;
        self.active_blocks.remove(&key);
        self.stopped_blocks.insert(key);
        Ok(content_block_stop(index))
    }

    fn require_block(&self, key: StreamBlockKey, event: &str) -> StreamTranslationResult<u32> {
        if self.stopped_blocks.contains(&key) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for {} after the Anthropic content block was already closed",
                block_key_context(key)
            )));
        }
        self.active_blocks.get(&key).map(|block| block.index).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for {} before the corresponding Responses start event",
                block_key_context(key)
            ))
        })
    }

    fn require_output_kind(
        &self,
        output_index: u32,
        expected: RegisteredOutputKind,
        event: &str,
    ) -> StreamTranslationResult<()> {
        if self.completed_outputs.contains(&output_index) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for output_index {output_index} after response.output_item.done"
            )));
        }
        match self.registered_outputs.get(&output_index).copied() {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for {actual} output_index {output_index}; expected {expected}"
            ))),
            None => Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for output_index {output_index} before response.output_item.added"
            ))),
        }
    }
}

fn block_kind_name(key: StreamBlockKey) -> &'static str {
    match key {
        StreamBlockKey::Text { .. } | StreamBlockKey::Refusal { .. } => "text",
        StreamBlockKey::Thinking { .. } | StreamBlockKey::Summary { .. } => "thinking",
        StreamBlockKey::ToolUse { .. } => "tool_use",
    }
}

fn block_output_index(key: StreamBlockKey) -> u32 {
    match key {
        StreamBlockKey::Text { output_index, .. }
        | StreamBlockKey::Refusal { output_index, .. }
        | StreamBlockKey::Thinking { output_index, .. }
        | StreamBlockKey::Summary { output_index, .. }
        | StreamBlockKey::ToolUse { output_index } => output_index,
    }
}

fn block_key_context(key: StreamBlockKey) -> String {
    match key {
        StreamBlockKey::Text {
            output_index,
            content_index,
        } => format!("text output_index {output_index} content_index {content_index}"),
        StreamBlockKey::Refusal {
            output_index,
            content_index,
        } => format!("refusal output_index {output_index} content_index {content_index}"),
        StreamBlockKey::Thinking {
            output_index,
            content_index,
        } => format!("reasoning output_index {output_index} content_index {content_index}"),
        StreamBlockKey::Summary {
            output_index,
            summary_index,
        } => {
            format!("reasoning summary output_index {output_index} summary_index {summary_index}")
        }
        StreamBlockKey::ToolUse { output_index } => format!("tool output_index {output_index}"),
    }
}
