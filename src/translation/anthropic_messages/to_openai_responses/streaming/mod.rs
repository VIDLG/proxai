//! `anthropic_messages -> openai_responses` streaming translator.
//!
//! Drives `state::StreamingState` and emits Responses `ResponseStreamEvent`s.
//! Per-content-block finalize/event construction lives in `output`; this
//! module owns only the event → state-mutation → emit loop and the carrier
//! boundary mapping to `StreamEvent`.

use crate::protocol::anthropic::messages::{ContentBlock, ContentBlockDelta, MessageStreamEvent};
use crate::protocol::openai_responses::Status;

use crate::translation::anthropic_messages::streaming::AnthropicInboundLifecycle;
use crate::translation::streaming::{
    SseStreamEnd, StreamEvent, StreamIdentity, StreamTranslationError, StreamTranslationResult,
    StreamingEventTranslator,
};

use super::types::response_id;

mod output;
mod state;

use state::StreamingState;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Default)]
pub(super) struct ResponsesStreamTranslator {
    sequence_number: u64,
    lifecycle: AnthropicInboundLifecycle<StreamingState>,
}

impl StreamingEventTranslator for ResponsesStreamTranslator {
    fn translate_event(&mut self, event: StreamEvent) -> StreamTranslationResult<Vec<StreamEvent>> {
        let parsed = self.lifecycle.parse_allowed_stream_event(event.data)?;
        let mut chunks = Vec::new();

        match parsed {
            MessageStreamEvent::MessageStart(event) => {
                let response_id = response_id(&event.message.id);
                let identity = StreamIdentity::new(response_id, event.message.model);
                let state = StreamingState::new(&identity, event.message.usage);
                self.lifecycle.begin_message_stream(identity, state)?;
                let sequence_number = self.next_sequence_number();
                let identity = self.lifecycle.stream_identity()?;
                let response = self
                    .lifecycle
                    .streaming_state()?
                    .response_snapshot(identity, Status::InProgress);
                chunks.push(output::response_created(sequence_number, response));
            }
            MessageStreamEvent::ContentBlockStart(event) => {
                let index = event.index;
                match event.content_block {
                    ContentBlock::Text(block) => {
                        let item_id = self.lifecycle.streaming_state_mut()?.next_message_item_id();
                        self.lifecycle.streaming_state_mut()?.register_text_block(
                            index,
                            item_id.clone(),
                            block.citations.clone(),
                        )?;
                        let sequence_number = self.next_sequence_number();
                        chunks.push(output::output_item_added(
                            sequence_number,
                            index,
                            output::message_item_initial(item_id),
                        ));
                        if !block.text.is_empty() {
                            let item_id = self
                                .lifecycle
                                .streaming_state_mut()?
                                .append_text_delta(index, &block.text)?;
                            self.lifecycle.streaming_phase_mut()?.mark_text();
                            let sequence_number = self.next_sequence_number();
                            chunks.push(output::output_text_delta(
                                sequence_number,
                                item_id,
                                index,
                                block.text,
                            ));
                        }
                    }
                    ContentBlock::Thinking(block) => {
                        let item_id = self
                            .lifecycle
                            .streaming_state_mut()?
                            .next_reasoning_item_id();
                        let sequence_number = self.next_sequence_number();
                        chunks.push(output::output_item_added(
                            sequence_number,
                            index,
                            output::reasoning_item_initial(item_id.to_string()),
                        ));
                        self.lifecycle
                            .streaming_state_mut()?
                            .register_thinking_block(index, item_id)?;
                        if !block.thinking.is_empty() {
                            let item_id = self
                                .lifecycle
                                .streaming_state_mut()?
                                .append_thinking_delta(index, &block.thinking)?;
                            self.lifecycle.streaming_phase_mut()?.mark_reasoning();
                            let sequence_number = self.next_sequence_number();
                            chunks.push(output::reasoning_text_delta(
                                sequence_number,
                                item_id,
                                index,
                                block.thinking,
                            ));
                        }
                    }
                    ContentBlock::RedactedThinking(block) => {
                        let item_id = self
                            .lifecycle
                            .streaming_state_mut()?
                            .next_reasoning_item_id();
                        let sequence_number = self.next_sequence_number();
                        chunks.push(output::output_item_added(
                            sequence_number,
                            index,
                            output::redacted_reasoning_item_initial(item_id.to_string()),
                        ));
                        self.lifecycle
                            .streaming_state_mut()?
                            .register_redacted_thinking_block(index, item_id, block.data.clone())?;
                        self.lifecycle.streaming_phase_mut()?.mark_reasoning();
                    }
                    ContentBlock::ToolUse(block) => {
                        let item_id = block.id;
                        let name = block.name;
                        self.lifecycle
                            .streaming_state_mut()?
                            .register_tool_use_block(index, item_id.clone(), name.clone())?;
                        self.lifecycle.streaming_phase_mut()?.mark_tool_use();
                        let sequence_number = self.next_sequence_number();
                        chunks.push(output::output_item_added(
                            sequence_number,
                            index,
                            output::tool_use_item_initial(item_id, name),
                        ));
                    }
                    ContentBlock::ToolResult(_)
                    | ContentBlock::ServerToolUse(_)
                    | ContentBlock::WebSearchToolResult(_)
                    | ContentBlock::WebFetchToolResult(_)
                    | ContentBlock::CodeExecutionToolResult(_)
                    | ContentBlock::BashCodeExecutionToolResult(_)
                    | ContentBlock::TextEditorCodeExecutionToolResult(_)
                    | ContentBlock::ToolSearchToolResult(_)
                    | ContentBlock::ContainerUpload(_) => {
                        return Err(StreamTranslationError::Semantic(
                            "Anthropic stream emitted content_block_start that Responses streaming cannot represent"
                                .to_string(),
                        ));
                    }
                }
            }
            MessageStreamEvent::ContentBlockDelta(event) => match event.delta {
                ContentBlockDelta::TextDelta(delta) => {
                    if !delta.text.is_empty() {
                        let item_id = self
                            .lifecycle
                            .streaming_state_mut()?
                            .append_text_delta(event.index, &delta.text)?;
                        self.lifecycle.streaming_phase_mut()?.mark_text();
                        let sequence_number = self.next_sequence_number();
                        chunks.push(output::output_text_delta(
                            sequence_number,
                            item_id,
                            event.index,
                            delta.text,
                        ));
                    } else {
                        self.lifecycle
                            .streaming_state_mut()?
                            .append_text_delta(event.index, "")?;
                    }
                }
                ContentBlockDelta::ThinkingDelta(delta) => {
                    if !delta.thinking.is_empty() {
                        let item_id = self
                            .lifecycle
                            .streaming_state_mut()?
                            .append_thinking_delta(event.index, &delta.thinking)?;
                        self.lifecycle.streaming_phase_mut()?.mark_reasoning();
                        let sequence_number = self.next_sequence_number();
                        chunks.push(output::reasoning_text_delta(
                            sequence_number,
                            item_id,
                            event.index,
                            delta.thinking,
                        ));
                    } else {
                        self.lifecycle
                            .streaming_state_mut()?
                            .append_thinking_delta(event.index, "")?;
                    }
                }
                ContentBlockDelta::InputJsonDelta(delta) => {
                    let item_id = self
                        .lifecycle
                        .streaming_state_mut()?
                        .append_tool_arguments_delta(event.index, &delta.partial_json)?;
                    let sequence_number = self.next_sequence_number();
                    chunks.push(output::tool_arguments_delta(
                        sequence_number,
                        item_id,
                        event.index,
                        delta.partial_json,
                    ));
                }
                ContentBlockDelta::SignatureDelta(_) => {
                    self.lifecycle
                        .streaming_state()?
                        .require_reasoning_signature_block(event.index)?;
                }
                ContentBlockDelta::CitationsDelta(_) => {
                    return Err(StreamTranslationError::Semantic(
                        "Anthropic stream emitted content_block_delta that Responses streaming cannot represent"
                            .to_string(),
                    ));
                }
            },
            MessageStreamEvent::ContentBlockStop(event) => {
                let index = event.index;
                let block = self.lifecycle.streaming_state_mut()?.stop_block(index)?;
                let sequence_number = self.next_sequence_number();
                let state = self.lifecycle.streaming_state_mut()?;
                let (item, content_done_chunks) = output::finalize_block(
                    block,
                    index,
                    sequence_number,
                    &mut state.text_char_offset,
                )?;

                // Accumulate the completed item so the terminal snapshot's
                // `output` field reflects what the stream actually produced,
                // and emit the protocol-mandated `output_item.done` to close
                // the lifecycle opened by `output_item.added`.
                let sequence_number = self.next_sequence_number();
                let done_event =
                    output::output_item_done_event(index, item.clone(), sequence_number);
                self.lifecycle
                    .streaming_state_mut()?
                    .output_items
                    .push(item);
                chunks.extend(content_done_chunks);
                chunks.push(done_event);
            }
            MessageStreamEvent::MessageDelta(event) => {
                let stop_reason = event.delta.stop_reason.ok_or_else(|| {
                    StreamTranslationError::Semantic(
                        "Anthropic stream emitted message_delta without stop_reason".to_string(),
                    )
                })?;
                let mut phase = self.lifecycle.take_streaming_phase()?;
                if !phase.emitted_any() {
                    return Err(StreamTranslationError::Semantic(
                        "Anthropic stream completed without Responses-representable content, thinking, or tool_use blocks"
                            .to_string(),
                    ));
                }
                let state = phase.state_mut();
                // MessageDelta carries an updated usage snapshot for the whole
                // message. Some fields are nullable in the wire model and may
                // be omitted; keep the last non-null value we already had.
                if let Some(input_tokens) = event.usage.input_tokens {
                    state.usage.input_tokens = input_tokens;
                }
                state.usage.output_tokens = event.usage.output_tokens;
                if let Some(cache_read) = event.usage.cache_read_input_tokens {
                    state.usage.cache_read_input_tokens = Some(cache_read);
                }
                if let Some(cache_creation) = event.usage.cache_creation_input_tokens {
                    state.usage.cache_creation_input_tokens = Some(cache_creation);
                }
                state.usage.output_tokens_details = event.usage.output_tokens_details;
                state.stop_reason = Some(stop_reason);
                self.lifecycle.receive_terminal_delta(phase);
            }
            MessageStreamEvent::MessageStop(_) => {
                let phase = self.lifecycle.take_terminal_phase()?;
                let state = phase.state();
                let status = state.terminal_response_status();
                let sequence_number = self.next_sequence_number();
                let identity = self.lifecycle.stream_identity()?;
                let response = state.response_snapshot(identity, status);
                self.lifecycle.stop();
                chunks.push(output::response_terminal(sequence_number, response, status));
            }
            MessageStreamEvent::Ping(_) => {}
        }

        chunks
            .into_iter()
            .map(|event| {
                let event_type = event.as_ref().to_string();
                StreamEvent::json(event_type, event)
            })
            .collect::<StreamTranslationResult<Vec<_>>>()
    }

    fn finish_stream(&mut self, end: SseStreamEnd) -> StreamTranslationResult<Vec<StreamEvent>> {
        if self.lifecycle.is_stopped() {
            return Ok(Vec::new());
        }

        Err(self.lifecycle.unexpected_stream_end_error(end))
    }
}

impl ResponsesStreamTranslator {
    fn next_sequence_number(&mut self) -> u64 {
        self.sequence_number += 1;
        self.sequence_number
    }
}
