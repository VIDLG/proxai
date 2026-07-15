//! Target-side state for
//! `openai_responses -> anthropic_messages` streaming translation.
//!
//! Envelope-level state (response identity, stream lifecycle phase) is owned
//! by
//! [`ResponsesInboundLifecycle`](crate::translation::openai_responses::streaming::ResponsesInboundLifecycle).
//! This module tracks only the Anthropic-message-specific projection:
//! whether `message_start` has been emitted, currently open content blocks,
//! and whether the stream has emitted its terminal `message_stop`.

use std::collections::BTreeMap;

use crate::protocol::anthropic::messages::{MessageDelta, MessageStreamEvent, StopReason, Usage};
use crate::protocol::openai_responses::{OutputContent, OutputItem, ResponseUsage, SummaryPart};

use crate::translation::TranslationScope;
use crate::translation::anthropic_messages::outbound::{
    content_block_stop, input_json_delta as input_json_delta_event,
    message_delta as message_delta_event, message_start as message_start_event, message_stop,
    text_block_start, text_delta as text_delta_event, thinking_block_start,
    thinking_delta as thinking_delta_event, tool_use_block_start,
};
use crate::translation::openai_responses::streaming::{
    ForwardedContent, ResponsesOutputSegmentKey,
};
use crate::translation::stream::{StreamIdentity, StreamTranslationError, StreamTranslationResult};

/// Per-stream Anthropic projection state.
///
/// Identity (message id + model) lives on the lifecycle's `StreamIdentity`;
/// this struct owns only Anthropic-specific projection state.
#[derive(Debug, Default)]
pub(super) struct StreamingState {
    message_started: bool,
    next_block_index: u32,
    blocks: BTreeMap<ResponsesOutputSegmentKey, ProjectedBlock>,
    saw_refusal: bool,
}

#[derive(Debug)]
enum ProjectedBlock {
    Active(ActiveBlock),
    Stopped,
}

#[derive(Debug)]
struct ActiveBlock {
    index: u32,
    forwarded: ForwardedContent,
    kind: ActiveBlockKind,
}

#[derive(Debug)]
enum ActiveBlockKind {
    TextLike,
    ToolUse(ToolInputState),
}

/// How a projected Anthropic `tool_use` block receives its input.
#[derive(Debug)]
enum ToolInputState {
    /// OpenAI function-call arguments are already JSON fragments and can be
    /// forwarded incrementally as Anthropic `input_json_delta` events.
    FunctionArguments,
    /// OpenAI custom-tool input is arbitrary text. Buffer it until the final
    /// snapshot so it can be encoded as one valid JSON string.
    CustomInput { buffered: String },
}

impl StreamingState {
    /// Emit `message_start` on the first call.
    pub(super) fn emit_message_start(
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

    pub(super) fn project_output_item_added(
        &mut self,
        output_index: u32,
        item: OutputItem,
        event: &str,
        scope: &TranslationScope,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        match item {
            OutputItem::Message(_) | OutputItem::Reasoning(_) => Ok(Vec::new()),
            OutputItem::FunctionCall(item) => {
                let key = ResponsesOutputSegmentKey::FunctionArguments { output_index };
                let start = self.open_tool_block(
                    key,
                    item.call_id,
                    item.name,
                    ToolInputState::FunctionArguments,
                )?;
                let mut events = vec![start];
                if !item.arguments.is_empty() {
                    let index = self.append_active_content(key, &item.arguments, event)?;
                    events.push(input_json_delta_event(index, item.arguments));
                }
                Ok(events)
            }
            OutputItem::CustomToolCall(item) => {
                let key = ResponsesOutputSegmentKey::CustomToolInput { output_index };
                let start = self.open_tool_block(
                    key,
                    item.call_id,
                    item.name,
                    ToolInputState::CustomInput {
                        buffered: item.input,
                    },
                )?;
                Ok(vec![start])
            }
            item @ (OutputItem::FileSearchCall(_)
            | OutputItem::WebSearchCall(_)
            | OutputItem::ImageGenerationCall(_)
            | OutputItem::CodeInterpreterCall(_)
            | OutputItem::McpCall(_)
            | OutputItem::McpListTools(_)
            | OutputItem::McpApprovalRequest(_)
            | OutputItem::ToolSearchCall(_)) => {
                scope.dropped(format!("Responses output item `{}`", item.as_ref()),
                    "Responses provider-hosted tool has no stable Anthropic server-tool wire mapping",
                );
                Ok(Vec::new())
            }
            item @ (OutputItem::ComputerCall(_)
            | OutputItem::LocalShellCall(_)
            | OutputItem::ShellCall(_)
            | OutputItem::ApplyPatchCall(_)) => {
                scope.dropped(format!("Responses output item `{}`", item.as_ref()),
                    "Responses tool call has no paired Anthropic tool definition and result-contract mapping",
                );
                Ok(Vec::new())
            }
            item @ (OutputItem::FunctionCallOutput(_)
            | OutputItem::CustomToolCallOutput(_)
            | OutputItem::ComputerCallOutput(_)
            | OutputItem::LocalShellCallOutput(_)
            | OutputItem::ShellCallOutput(_)
            | OutputItem::ApplyPatchCallOutput(_)
            | OutputItem::McpApprovalResponse(_)
            | OutputItem::ToolSearchOutput(_)) => {
                scope.dropped(format!("Responses output item `{}`", item.as_ref()),
                    "Responses tool result is request-side Anthropic content, not an outbound message stream block",
                );
                Ok(Vec::new())
            }
            item @ OutputItem::Compaction(_) => {
                scope.dropped(format!("Responses output item `{}`", item.as_ref()),
                    "Responses compaction item is internal transcript state with no Anthropic Messages stream representation",
                );
                Ok(Vec::new())
            }
        }
    }

    /// Verify that the target projection for a completed source output item is closed.
    ///
    /// Source-side `response.output_item.done` ordering, identity, and kind are
    /// validated by `ResponsesInboundLifecycle` before this pair state observes it.
    pub(super) fn ensure_output_item_projection_closed(
        &self,
        output_index: u32,
        event: &str,
    ) -> StreamTranslationResult<()> {
        if let Some((key, _)) = self.blocks.iter().find(|(key, block)| {
            matches!(block, ProjectedBlock::Active(_)) && key.output_index() == output_index
        }) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} before {} was closed",
                key.context()
            )));
        }
        Ok(())
    }

    pub(super) fn project_content_part_added(
        &mut self,
        output_index: u32,
        content_index: u32,
        part: OutputContent,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        match part {
            OutputContent::OutputText(part) => {
                let key = ResponsesOutputSegmentKey::Text {
                    output_index,
                    content_index,
                };
                Ok(vec![self.open_text_block(key, part.text)?])
            }
            OutputContent::Refusal(part) => {
                self.saw_refusal = true;
                let key = ResponsesOutputSegmentKey::Refusal {
                    output_index,
                    content_index,
                };
                Ok(vec![self.open_text_block(key, part.refusal)?])
            }
            OutputContent::ReasoningText(part) => {
                let key = ResponsesOutputSegmentKey::ReasoningText {
                    output_index,
                    content_index,
                };
                self.open_thinking_block(key, part.text, event)
            }
        }
    }

    pub(super) fn project_content_part_done(
        &mut self,
        output_index: u32,
        content_index: u32,
        part: OutputContent,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let (key, final_content) = match part {
            OutputContent::OutputText(part) => (
                ResponsesOutputSegmentKey::Text {
                    output_index,
                    content_index,
                },
                part.text,
            ),
            OutputContent::Refusal(part) => {
                self.saw_refusal = true;
                (
                    ResponsesOutputSegmentKey::Refusal {
                        output_index,
                        content_index,
                    },
                    part.refusal,
                )
            }
            OutputContent::ReasoningText(part) => (
                ResponsesOutputSegmentKey::ReasoningText {
                    output_index,
                    content_index,
                },
                part.text,
            ),
        };

        // OpenAI-compatible upstreams differ on whether `*.done` or
        // `content_part.done` closes the semantic part. If the protocol-specific
        // done event already closed it, this envelope event is only validation.
        if matches!(self.blocks.get(&key), Some(ProjectedBlock::Stopped)) {
            return Ok(Vec::new());
        }
        self.finish_text_like_block(key, final_content, event)
    }

    pub(super) fn project_output_text_delta(
        &mut self,
        output_index: u32,
        content_index: u32,
        delta: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let key = ResponsesOutputSegmentKey::Text {
            output_index,
            content_index,
        };
        self.forward_text_like_delta(key, delta, event)
    }

    pub(super) fn project_output_text_done(
        &mut self,
        output_index: u32,
        content_index: u32,
        text: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.finish_text_like_block(
            ResponsesOutputSegmentKey::Text {
                output_index,
                content_index,
            },
            text,
            event,
        )
    }

    pub(super) fn project_refusal_delta(
        &mut self,
        output_index: u32,
        content_index: u32,
        delta: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.saw_refusal = true;
        let key = ResponsesOutputSegmentKey::Refusal {
            output_index,
            content_index,
        };
        self.forward_text_like_delta(key, delta, event)
    }

    pub(super) fn project_refusal_done(
        &mut self,
        output_index: u32,
        content_index: u32,
        refusal: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.saw_refusal = true;
        self.finish_text_like_block(
            ResponsesOutputSegmentKey::Refusal {
                output_index,
                content_index,
            },
            refusal,
            event,
        )
    }

    pub(super) fn project_reasoning_summary_part_added(
        &mut self,
        output_index: u32,
        summary_index: u32,
        part: SummaryPart,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let SummaryPart::SummaryText(part) = part;
        self.open_thinking_block(
            ResponsesOutputSegmentKey::ReasoningSummary {
                output_index,
                summary_index,
            },
            part.text,
            event,
        )
    }

    pub(super) fn project_reasoning_summary_text_delta(
        &mut self,
        output_index: u32,
        summary_index: u32,
        delta: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let key = ResponsesOutputSegmentKey::ReasoningSummary {
            output_index,
            summary_index,
        };
        self.forward_text_like_delta(key, delta, event)
    }

    pub(super) fn project_reasoning_summary_text_done(
        &mut self,
        output_index: u32,
        summary_index: u32,
        text: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let key = ResponsesOutputSegmentKey::ReasoningSummary {
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

    pub(super) fn project_reasoning_summary_part_done(
        &mut self,
        output_index: u32,
        summary_index: u32,
        part: SummaryPart,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let SummaryPart::SummaryText(part) = part;
        self.finish_text_like_block(
            ResponsesOutputSegmentKey::ReasoningSummary {
                output_index,
                summary_index,
            },
            part.text,
            event,
        )
    }

    pub(super) fn project_reasoning_text_delta(
        &mut self,
        output_index: u32,
        content_index: u32,
        delta: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let key = ResponsesOutputSegmentKey::ReasoningText {
            output_index,
            content_index,
        };
        self.forward_text_like_delta(key, delta, event)
    }

    pub(super) fn project_reasoning_text_done(
        &mut self,
        output_index: u32,
        content_index: u32,
        text: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        self.finish_text_like_block(
            ResponsesOutputSegmentKey::ReasoningText {
                output_index,
                content_index,
            },
            text,
            event,
        )
    }

    pub(super) fn project_function_call_arguments_delta(
        &mut self,
        output_index: u32,
        delta: String,
        event: &str,
    ) -> StreamTranslationResult<MessageStreamEvent> {
        let key = ResponsesOutputSegmentKey::FunctionArguments { output_index };
        let index = self.append_active_content(key, &delta, event)?;
        Ok(input_json_delta_event(index, delta))
    }

    pub(super) fn project_function_call_arguments_done(
        &mut self,
        output_index: u32,
        arguments: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let key = ResponsesOutputSegmentKey::FunctionArguments { output_index };
        let mut events = Vec::new();
        if let Some((index, suffix)) = self.reconcile_active_content(key, &arguments, event)? {
            events.push(input_json_delta_event(index, suffix));
        }
        events.push(self.stop_block(key, event)?);
        Ok(events)
    }

    pub(super) fn project_custom_tool_call_input_delta(
        &mut self,
        output_index: u32,
        delta: &str,
        event: &str,
    ) -> StreamTranslationResult<()> {
        let key = ResponsesOutputSegmentKey::CustomToolInput { output_index };
        let block = self.require_active_block_mut(key, event)?;
        let ActiveBlockKind::ToolUse(ToolInputState::CustomInput { buffered }) = &mut block.kind
        else {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for non-custom tool output_index {output_index}"
            )));
        };
        buffered.push_str(delta);
        Ok(())
    }

    pub(super) fn project_custom_tool_call_input_done(
        &mut self,
        output_index: u32,
        input: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let key = ResponsesOutputSegmentKey::CustomToolInput { output_index };
        let block = self.require_active_block_mut(key, event)?;
        let ActiveBlockKind::ToolUse(ToolInputState::CustomInput { buffered }) = &mut block.kind
        else {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for non-custom tool output_index {output_index}"
            )));
        };
        let observed = std::mem::take(buffered);
        if !input.starts_with(&observed) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} whose final custom tool input diverged from streamed input for output_index {output_index}"
            )));
        }

        let encoded = serde_json::to_string(&input)?;
        let index = self.append_active_content(key, &encoded, event)?;
        Ok(vec![
            input_json_delta_event(index, encoded),
            self.stop_block(key, event)?,
        ])
    }

    pub(super) fn finish_message(
        &self,
        stop_reason: StopReason,
        usage: Option<&ResponseUsage>,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        if let Some((key, _)) = self
            .blocks
            .iter()
            .find(|(_, block)| matches!(block, ProjectedBlock::Active(_)))
        {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream completed before {} was closed",
                key.context()
            )));
        }
        let stop_reason = if self.saw_refusal && matches!(stop_reason, StopReason::EndTurn) {
            StopReason::Refusal
        } else {
            stop_reason
        };

        Ok(vec![
            message_delta_event(
                MessageDelta {
                    container: None.into(),
                    stop_details: None.into(),
                    stop_reason: Some(stop_reason).into(),
                    stop_sequence: None.into(),
                },
                usage.map(Into::into).unwrap_or_default(),
            ),
            message_stop(),
        ])
    }

    fn forward_text_like_delta(
        &mut self,
        key: ResponsesOutputSegmentKey,
        delta: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let mut events = self.ensure_text_like_block_started(key, event)?;
        let index = self.append_active_content(key, &delta, event)?;
        events.push(text_like_delta_event(key, index, delta));
        Ok(events)
    }

    fn finish_text_like_block(
        &mut self,
        key: ResponsesOutputSegmentKey,
        final_content: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let mut events = self.ensure_text_like_block_started(key, event)?;
        if let Some((index, suffix)) = self.reconcile_active_content(key, &final_content, event)? {
            events.push(text_like_delta_event(key, index, suffix));
        }
        events.push(self.stop_block(key, event)?);
        Ok(events)
    }

    fn ensure_text_like_block_started(
        &mut self,
        key: ResponsesOutputSegmentKey,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        if self.has_active_block(key) {
            return Ok(Vec::new());
        }
        if key.is_reasoning() {
            self.open_thinking_block(key, String::new(), event)
        } else {
            Ok(vec![self.open_text_block(key, String::new())?])
        }
    }

    fn open_text_block(
        &mut self,
        key: ResponsesOutputSegmentKey,
        initial: String,
    ) -> StreamTranslationResult<MessageStreamEvent> {
        let index = self.open_block_index(key, initial.clone(), ActiveBlockKind::TextLike)?;
        Ok(text_block_start(index, initial))
    }

    fn open_thinking_block(
        &mut self,
        key: ResponsesOutputSegmentKey,
        initial: String,
        event: &str,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let index = self.open_block_index(key, String::new(), ActiveBlockKind::TextLike)?;
        let mut events = vec![thinking_block_start(index)];
        if !initial.is_empty() {
            self.append_active_content(key, &initial, event)?;
            events.push(thinking_delta_event(index, initial));
        }
        Ok(events)
    }

    fn open_tool_block(
        &mut self,
        key: ResponsesOutputSegmentKey,
        item_id: String,
        name: String,
        tool_input: ToolInputState,
    ) -> StreamTranslationResult<MessageStreamEvent> {
        let index =
            self.open_block_index(key, String::new(), ActiveBlockKind::ToolUse(tool_input))?;
        Ok(tool_use_block_start(index, item_id, name))
    }

    fn open_block_index(
        &mut self,
        key: ResponsesOutputSegmentKey,
        forwarded: String,
        kind: ActiveBlockKind,
    ) -> StreamTranslationResult<u32> {
        let anthropic_block_kind = if key.is_reasoning() {
            "thinking"
        } else if key.is_tool_input() {
            "tool_use"
        } else {
            "text"
        };
        if self.blocks.contains_key(&key) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream attempted to open duplicate Anthropic {} content block for {}",
                anthropic_block_kind,
                key.context()
            )));
        }
        let index = self.next_block_index;
        self.next_block_index += 1;
        self.blocks.insert(
            key,
            ProjectedBlock::Active(ActiveBlock {
                index,
                forwarded: forwarded.into(),
                kind,
            }),
        );
        Ok(index)
    }

    fn has_active_block(&self, key: ResponsesOutputSegmentKey) -> bool {
        matches!(self.blocks.get(&key), Some(ProjectedBlock::Active(_)))
    }

    fn append_active_content(
        &mut self,
        key: ResponsesOutputSegmentKey,
        delta: &str,
        event: &str,
    ) -> StreamTranslationResult<u32> {
        let block = self.require_active_block_mut(key, event)?;
        block.forwarded.append(delta);
        Ok(block.index)
    }

    fn reconcile_active_content(
        &mut self,
        key: ResponsesOutputSegmentKey,
        final_content: &str,
        event: &str,
    ) -> StreamTranslationResult<Option<(u32, String)>> {
        let block = self.require_active_block_mut(key, event)?;
        let suffix = block
            .forwarded
            .reconcile_snapshot(final_content)
            .map_err(|_| {
                StreamTranslationError::Semantic(format!(
                    "Responses stream emitted {event} whose final content diverged from streamed content for {}",
                    key.context()
                ))
            })?;
        Ok(suffix.map(|suffix| (block.index, suffix)))
    }

    fn stop_block(
        &mut self,
        key: ResponsesOutputSegmentKey,
        event: &str,
    ) -> StreamTranslationResult<MessageStreamEvent> {
        let index = self.require_active_block(key, event)?.index;
        self.blocks.insert(key, ProjectedBlock::Stopped);
        Ok(content_block_stop(index))
    }

    fn require_active_block(
        &self,
        key: ResponsesOutputSegmentKey,
        event: &str,
    ) -> StreamTranslationResult<&ActiveBlock> {
        match self.blocks.get(&key) {
            Some(ProjectedBlock::Active(block)) => Ok(block),
            Some(ProjectedBlock::Stopped) => Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for {} after the Anthropic content block was already closed",
                key.context()
            ))),
            None => Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for {} before the corresponding Responses start event",
                key.context()
            ))),
        }
    }

    fn require_active_block_mut(
        &mut self,
        key: ResponsesOutputSegmentKey,
        event: &str,
    ) -> StreamTranslationResult<&mut ActiveBlock> {
        match self.blocks.get_mut(&key) {
            Some(ProjectedBlock::Active(block)) => Ok(block),
            Some(ProjectedBlock::Stopped) => Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for {} after the Anthropic content block was already closed",
                key.context()
            ))),
            None => Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for {} before the corresponding Responses start event",
                key.context()
            ))),
        }
    }
}

fn text_like_delta_event(
    key: ResponsesOutputSegmentKey,
    index: u32,
    content: String,
) -> MessageStreamEvent {
    if key.is_reasoning() {
        thinking_delta_event(index, content)
    } else {
        text_delta_event(index, content)
    }
}
