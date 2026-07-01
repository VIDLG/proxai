use std::collections::BTreeMap;

use crate::protocol::anthropic::messages::{
    ContentBlock, ContentBlockDelta, MessageStreamEvent, StopReason, TextBlock, TextCitation, Usage,
};
use crate::protocol::openai_responses::{
    AssistantRole, FunctionToolCall, OutputItem, OutputMessage, OutputMessageContent, OutputStatus,
    OutputTextContent, ReasoningItem, ReasoningItemContent, ReasoningTextContent, Response,
    ResponseCompletedEvent, ResponseCreatedEvent, ResponseFunctionCallArgumentsDeltaEvent,
    ResponseFunctionCallArgumentsDoneEvent, ResponseIncompleteEvent, ResponseOutputItemAddedEvent,
    ResponseOutputItemDoneEvent, ResponseReasoningTextDeltaEvent, ResponseReasoningTextDoneEvent,
    ResponseStreamEvent, ResponseTextDeltaEvent, ResponseTextDoneEvent, ResponseUsage, Status,
};

use crate::translation::anthropic_messages::streaming::AnthropicInboundLifecycle;
use crate::translation::streaming::{
    SseStreamEnd, StreamEvent, StreamIdentity, StreamTranslationError, StreamTranslationResult,
    StreamingEventTranslator,
};

use super::citations::text_block_annotations;
use super::ids::OutputItemIdAllocator;
use super::types::{incomplete_details_from_stop_reason, response_id};

#[derive(Debug, Default)]
pub(super) struct ResponsesStreamTranslator {
    sequence_number: u64,
    lifecycle: AnthropicInboundLifecycle<StreamingState>,
}

#[derive(Debug)]
struct StreamingState {
    usage: Usage,
    item_ids: OutputItemIdAllocator,
    stop_reason: Option<StopReason>,
    blocks: BTreeMap<u32, StreamBlock>,
    output_items: Vec<OutputItem>,
    // Cumulative character count of all completed text items so far. Used as
    // the base offset when translating a TextBlock's citations to Responses
    // annotations, which use character indices relative to the full text
    // output (matching the non-streaming path in response.rs).
    text_char_offset: usize,
}

/// Per-block in-flight state, keyed by Anthropic `content_block.index` in
/// `StreamingState::blocks`.
///
/// Each variant's fields split into two roles:
///
/// - **Accumulated content** (`text`, `arguments`): initialized empty by
///   `register_*_block`, filled by the matching `append_*_delta` as
///   `content_block_delta` events arrive. Read at `content_block_stop` time
///   to build the finalized `OutputItem`.
/// - **One-shot metadata** (`item_id`, `citations`, `name`, `data`): attached
///   once at registration, never mutated, consumed at stop time. These are
///   fields whose Anthropic-side values arrive in full on `content_block_start`
///   and have no delta equivalent proxai supports.
#[derive(Debug, Clone, PartialEq, Eq)]
enum StreamBlock {
    Text {
        item_id: String,
        text: String,
        citations: Option<Vec<TextCitation>>,
    },
    Thinking {
        item_id: String,
        text: String,
    },
    RedactedThinking {
        item_id: String,
        data: String,
    },
    ToolUse {
        item_id: String,
        name: String,
        arguments: String,
    },
}

impl StreamingState {
    fn new(identity: &StreamIdentity, usage: Usage) -> Self {
        let item_ids = OutputItemIdAllocator::new(identity.id().to_string());
        Self {
            usage,
            item_ids,
            stop_reason: None,
            blocks: BTreeMap::new(),
            output_items: Vec::new(),
            text_char_offset: 0,
        }
    }

    fn next_message_item_id(&mut self) -> String {
        self.item_ids.message()
    }

    fn next_reasoning_item_id(&mut self) -> String {
        self.item_ids.reasoning()
    }

    fn terminal_response_status(&self) -> Status {
        // Match the non-streaming conversion in types.rs: refusal is Failed,
        // max_tokens is Incomplete, everything else is Completed. Reusing the
        // shared `From<StopReason> for Status` impl keeps streaming and
        // non-streaming terminal status in lockstep.
        self.stop_reason
            .map(Status::from)
            .unwrap_or(Status::Completed)
    }

    fn response_snapshot(&self, identity: &StreamIdentity, status: Status) -> Response {
        let incomplete_details = incomplete_details_from_stop_reason(self.stop_reason);
        Response {
            background: None,
            billing: None,
            conversation: None,
            created_at: 0,
            completed_at: None,
            error: None,
            id: identity.id().to_string(),
            incomplete_details,
            instructions: None,
            max_output_tokens: None,
            metadata: None,
            model: identity.model().to_string(),
            object: "response".to_string(),
            output: self.output_items.clone(),
            parallel_tool_calls: None,
            previous_response_id: None,
            prompt: None,
            prompt_cache_key: None,
            prompt_cache_retention: None,
            reasoning: None,
            safety_identifier: None,
            service_tier: self.usage.service_tier.and_then(Into::into),
            status,
            temperature: None,
            text: None,
            tool_choice: None,
            tools: None,
            top_logprobs: None,
            top_p: None,
            truncation: None,
            usage: Some(ResponseUsage::from(&self.usage)),
        }
    }

    /// Open a text block slot.
    ///
    /// `text` is intentionally initialized to an empty string. Anthropic
    /// delivers text incrementally via `content_block_delta.text_delta`
    /// events (and may also send an initial non-empty `text` on
    /// `content_block_start`). All text — including any text present on
    /// `content_block_start` — is accumulated via `append_text_delta`, which
    /// keeps the start/delta paths uniform and avoids duplicating push_str
    /// logic here.
    ///
    /// `citations` is attached directly because, in the wire shape proxai
    /// supports, citations arrive once and in full on `content_block_start`
    /// (the `citations_delta` event variant is rejected as unrepresentable).
    /// They never change after registration, so they live as block metadata
    /// rather than as accumulated content. They are consumed at
    /// `content_block_stop` time by `text_block_annotations`.
    ///
    /// `mark_text` is NOT called here: `content_block_start.text` may be
    /// empty, and the representable-output tracker must only flip when the
    /// stream has actually produced Responses-representable text. The caller
    /// marks the phase output when appending a non-empty text delta.
    fn register_text_block(
        &mut self,
        block_index: u32,
        item_id: String,
        citations: Option<Vec<TextCitation>>,
    ) -> StreamTranslationResult<()> {
        self.register_block(
            block_index,
            StreamBlock::Text {
                item_id,
                text: String::new(),
                citations,
            },
        )
    }

    /// Open a thinking block slot. See `register_text_block` for why `text`
    /// starts empty and why `mark_reasoning` is deferred until the caller
    /// appends a non-empty thinking delta.
    fn register_thinking_block(
        &mut self,
        block_index: u32,
        item_id: String,
    ) -> StreamTranslationResult<()> {
        self.register_block(
            block_index,
            StreamBlock::Thinking {
                item_id,
                text: String::new(),
            },
        )
    }

    /// Open a redacted-thinking block slot and immediately mark reasoning as
    /// emitted.
    ///
    /// Unlike text/thinking, redacted thinking has no streamed delta events
    /// of any kind in the Anthropic wire model — the entire opaque `data`
    /// blob arrives on `content_block_start` and never changes afterwards.
    /// There is therefore no `append_redacted_thinking_delta` companion, and
    /// `data` is attached here rather than accumulated. Because the payload
    /// The caller marks reasoning immediately after registration because the
    /// payload is non-empty by construction at registration time.
    fn register_redacted_thinking_block(
        &mut self,
        block_index: u32,
        item_id: String,
        data: String,
    ) -> StreamTranslationResult<()> {
        self.register_block(block_index, StreamBlock::RedactedThinking { item_id, data })?;
        Ok(())
    }

    /// Open a tool-use block slot and immediately mark tool use as emitted.
    ///
    /// `arguments` starts empty: the JSON argument string is built up by
    /// subsequent `input_json_delta` events via `append_tool_arguments_delta`.
    ///
    /// The caller marks tool use immediately after registration because `id`
    /// and `name` are mandatory and always present on `content_block_start`.
    fn register_tool_use_block(
        &mut self,
        block_index: u32,
        item_id: String,
        name: String,
    ) -> StreamTranslationResult<()> {
        self.register_block(
            block_index,
            StreamBlock::ToolUse {
                item_id,
                name,
                arguments: String::new(),
            },
        )?;
        Ok(())
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

    fn append_text_delta(
        &mut self,
        block_index: u32,
        delta: &str,
    ) -> StreamTranslationResult<String> {
        match self.blocks.get_mut(&block_index) {
            Some(StreamBlock::Text { item_id, text, .. }) => {
                text.push_str(delta);
                Ok(item_id.clone())
            }
            Some(
                StreamBlock::Thinking { .. }
                | StreamBlock::RedactedThinking { .. }
                | StreamBlock::ToolUse { .. },
            ) => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted text_delta for incompatible content block index {block_index}"
            ))),
            None => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted text_delta for unopened content block index {block_index}"
            ))),
        }
    }

    fn append_thinking_delta(
        &mut self,
        block_index: u32,
        delta: &str,
    ) -> StreamTranslationResult<String> {
        match self.blocks.get_mut(&block_index) {
            Some(StreamBlock::Thinking { item_id, text }) => {
                text.push_str(delta);
                Ok(item_id.clone())
            }
            Some(
                StreamBlock::Text { .. }
                | StreamBlock::RedactedThinking { .. }
                | StreamBlock::ToolUse { .. },
            ) => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted thinking_delta for incompatible content block index {block_index}"
            ))),
            None => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted thinking_delta for unopened content block index {block_index}"
            ))),
        }
    }

    fn append_tool_arguments_delta(
        &mut self,
        block_index: u32,
        delta: &str,
    ) -> StreamTranslationResult<String> {
        match self.blocks.get_mut(&block_index) {
            Some(StreamBlock::ToolUse {
                item_id, arguments, ..
            }) => {
                arguments.push_str(delta);
                Ok(item_id.clone())
            }
            Some(
                StreamBlock::Text { .. }
                | StreamBlock::Thinking { .. }
                | StreamBlock::RedactedThinking { .. },
            ) => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted input_json_delta for incompatible content block index {block_index}"
            ))),
            None => Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted input_json_delta for unopened content block index {block_index}"
            ))),
        }
    }

    fn require_reasoning_signature_block(&self, block_index: u32) -> StreamTranslationResult<()> {
        let Some(actual) = self.blocks.get(&block_index) else {
            return Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted signature_delta for unopened content block index {block_index}"
            )));
        };
        if !matches!(actual, StreamBlock::Thinking { .. }) {
            return Err(StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted signature_delta for incompatible content block index {block_index}"
            )));
        }
        Ok(())
    }

    fn stop_block(&mut self, block_index: u32) -> StreamTranslationResult<StreamBlock> {
        self.blocks.remove(&block_index).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted content_block_stop for unopened content block index {block_index}"
            ))
        })
    }
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
                chunks.push(ResponseStreamEvent::ResponseCreated(ResponseCreatedEvent {
                    sequence_number,
                    response,
                }));
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
                        chunks.push(ResponseStreamEvent::ResponseOutputItemAdded(
                            ResponseOutputItemAddedEvent {
                                sequence_number,
                                output_index: index,
                                item: OutputItem::Message(OutputMessage {
                                    id: item_id,
                                    role: AssistantRole::Assistant,
                                    status: OutputStatus::InProgress,
                                    content: Vec::new(),
                                    phase: None,
                                }),
                            },
                        ));
                        if !block.text.is_empty() {
                            let item_id = self
                                .lifecycle
                                .streaming_state_mut()?
                                .append_text_delta(index, &block.text)?;
                            self.lifecycle.streaming_phase_mut()?.mark_text();
                            let sequence_number = self.next_sequence_number();
                            chunks.push(ResponseStreamEvent::ResponseOutputTextDelta(
                                ResponseTextDeltaEvent {
                                    sequence_number,
                                    item_id,
                                    output_index: index,
                                    content_index: 0,
                                    delta: block.text,
                                    logprobs: None,
                                },
                            ));
                        }
                    }
                    ContentBlock::Thinking(block) => {
                        let item_id = self
                            .lifecycle
                            .streaming_state_mut()?
                            .next_reasoning_item_id();
                        let sequence_number = self.next_sequence_number();
                        chunks.push(ResponseStreamEvent::ResponseOutputItemAdded(
                            ResponseOutputItemAddedEvent {
                                sequence_number,
                                output_index: index,
                                item: OutputItem::Reasoning(ReasoningItem {
                                    id: Some(item_id.to_string()),
                                    summary: Vec::new(),
                                    content: Some(Vec::new()),
                                    encrypted_content: None,
                                    status: Some(OutputStatus::InProgress),
                                }),
                            },
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
                            chunks.push(ResponseStreamEvent::ResponseReasoningTextDelta(
                                ResponseReasoningTextDeltaEvent {
                                    sequence_number,
                                    item_id,
                                    output_index: index,
                                    content_index: 0,
                                    delta: block.thinking,
                                },
                            ));
                        }
                    }
                    ContentBlock::RedactedThinking(block) => {
                        let item_id = self
                            .lifecycle
                            .streaming_state_mut()?
                            .next_reasoning_item_id();
                        let sequence_number = self.next_sequence_number();
                        chunks.push(ResponseStreamEvent::ResponseOutputItemAdded(
                            ResponseOutputItemAddedEvent {
                                sequence_number,
                                output_index: index,
                                item: OutputItem::Reasoning(ReasoningItem {
                                    id: Some(item_id.to_string()),
                                    summary: Vec::new(),
                                    content: None,
                                    // Placeholder; the real data arrives with content_block_stop.
                                    encrypted_content: None,
                                    status: Some(OutputStatus::InProgress),
                                }),
                            },
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
                        chunks.push(ResponseStreamEvent::ResponseOutputItemAdded(
                            ResponseOutputItemAddedEvent {
                                sequence_number,
                                output_index: index,
                                item: OutputItem::FunctionCall(FunctionToolCall {
                                    id: Some(item_id.clone()),
                                    call_id: item_id,
                                    name,
                                    arguments: String::new(),
                                    status: Some(OutputStatus::InProgress),
                                    namespace: None,
                                }),
                            },
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
                        chunks.push(ResponseStreamEvent::ResponseOutputTextDelta(
                            ResponseTextDeltaEvent {
                                sequence_number,
                                item_id,
                                output_index: event.index,
                                content_index: 0,
                                delta: delta.text,
                                logprobs: None,
                            },
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
                        chunks.push(ResponseStreamEvent::ResponseReasoningTextDelta(
                            ResponseReasoningTextDeltaEvent {
                                sequence_number,
                                item_id,
                                output_index: event.index,
                                content_index: 0,
                                delta: delta.thinking,
                            },
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
                    chunks.push(ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(
                        ResponseFunctionCallArgumentsDeltaEvent {
                            sequence_number,
                            item_id,
                            output_index: event.index,
                            delta: delta.partial_json,
                        },
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
                let (item, content_done_chunks) = match self
                    .lifecycle
                    .streaming_state_mut()?
                    .stop_block(index)?
                {
                    StreamBlock::Text {
                        item_id,
                        text,
                        citations,
                    } => {
                        let sequence_number = self.next_sequence_number();
                        let done =
                            ResponseStreamEvent::ResponseOutputTextDone(ResponseTextDoneEvent {
                                sequence_number,
                                item_id: item_id.clone(),
                                output_index: index,
                                content_index: 0,
                                text: text.clone(),
                                logprobs: None,
                            });
                        // Translate Anthropic citations to Responses URL
                        // annotations using the cumulative character offset
                        // of all previous text items, mirroring the
                        // non-streaming conversion in response.rs.
                        let synthetic_block = TextBlock {
                            text: text.clone(),
                            citations,
                        };
                        let state = self.lifecycle.streaming_state_mut()?;
                        let annotations =
                            text_block_annotations(&synthetic_block, state.text_char_offset);
                        state.text_char_offset =
                            state.text_char_offset.saturating_add(text.chars().count());
                        let item = OutputItem::Message(OutputMessage {
                            id: item_id,
                            role: AssistantRole::Assistant,
                            status: OutputStatus::Completed,
                            content: vec![OutputMessageContent::OutputText(OutputTextContent {
                                text,
                                annotations,
                                logprobs: None,
                            })],
                            phase: None,
                        });
                        (item, vec![done])
                    }
                    StreamBlock::Thinking { item_id, text } => {
                        let sequence_number = self.next_sequence_number();
                        let done = ResponseStreamEvent::ResponseReasoningTextDone(
                            ResponseReasoningTextDoneEvent {
                                sequence_number,
                                item_id: item_id.clone(),
                                output_index: index,
                                content_index: 0,
                                text: text.clone(),
                            },
                        );
                        let item = OutputItem::Reasoning(ReasoningItem {
                            id: Some(item_id),
                            summary: Vec::new(),
                            content: Some(vec![ReasoningItemContent::ReasoningText(
                                ReasoningTextContent { text },
                            )]),
                            encrypted_content: None,
                            status: Some(OutputStatus::Completed),
                        });
                        (item, vec![done])
                    }
                    StreamBlock::RedactedThinking { item_id, data } => {
                        // Redacted thinking has no streamed text deltas; only
                        // the lifecycle close events are emitted. The
                        // `encrypted_content` field carries the opaque payload
                        // that non-streaming translation also surfaces.
                        let item = OutputItem::Reasoning(ReasoningItem {
                            id: Some(item_id),
                            summary: Vec::new(),
                            content: None,
                            encrypted_content: Some(data),
                            status: Some(OutputStatus::Completed),
                        });
                        (item, Vec::new())
                    }
                    StreamBlock::ToolUse {
                        item_id,
                        name,
                        arguments,
                    } => {
                        let sequence_number = self.next_sequence_number();
                        let done = ResponseStreamEvent::ResponseFunctionCallArgumentsDone(
                            ResponseFunctionCallArgumentsDoneEvent {
                                sequence_number,
                                item_id: item_id.clone(),
                                output_index: index,
                                name: Some(name.clone()),
                                arguments: arguments.clone(),
                            },
                        );
                        let item = OutputItem::FunctionCall(FunctionToolCall {
                            id: Some(item_id.clone()),
                            call_id: item_id,
                            name,
                            arguments,
                            status: Some(OutputStatus::Completed),
                            namespace: None,
                        });
                        (item, vec![done])
                    }
                };

                // Accumulate the completed item so the terminal snapshot's
                // `output` field reflects what the stream actually produced,
                // and emit the protocol-mandated `output_item.done` to close
                // the lifecycle opened by `output_item.added`.
                let sequence_number = self.next_sequence_number();
                let done_event =
                    ResponseStreamEvent::ResponseOutputItemDone(ResponseOutputItemDoneEvent {
                        sequence_number,
                        output_index: index,
                        item: item.clone(),
                    });
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
                let event = match status {
                    Status::Incomplete => {
                        ResponseStreamEvent::ResponseIncomplete(ResponseIncompleteEvent {
                            sequence_number,
                            response,
                        })
                    }
                    _ => ResponseStreamEvent::ResponseCompleted(ResponseCompletedEvent {
                        sequence_number,
                        response,
                    }),
                };
                chunks.push(event);
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

#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
