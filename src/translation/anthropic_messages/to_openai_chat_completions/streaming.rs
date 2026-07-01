use axum::body::Bytes;
use std::collections::BTreeMap;

use crate::protocol::anthropic::messages::{
    ContentBlock, ContentBlockDelta, InputJsonDelta, MessageDelta, MessageStartEvent,
    MessageStreamEvent, StopReason, ToolUseBlock,
};
use crate::protocol::openai::chat_completions::{
    ChatChoiceStream, ChatCompletionMessageToolCallChunk, ChatCompletionStreamResponseDelta,
    CompletionUsage, CreateChatCompletionStreamResponse, FinishReason, FunctionCallStream,
    FunctionType, Role,
};
use crate::sse::{SseEvent, done_sentinel_bytes};
use crate::translation::anthropic_messages::streaming::AnthropicInboundLifecycle;
use crate::translation::streaming::{
    SseStreamEnd, StreamIdentity, StreamTranslationError, StreamTranslationResult,
    StreamingEventTranslator, encode_sse_json,
};

#[derive(Debug, Default)]
pub(super) struct ChatCompletionStreamTranslator {
    lifecycle: AnthropicInboundLifecycle<StreamingState>,
}

#[derive(Debug)]
struct StreamingState {
    blocks: BTreeMap<u32, StreamBlock>,
    next_tool_call_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamBlock {
    Text,
    ToolUse { chat_tool_index: u32 },
    Thinking,
    Ignored,
}

impl StreamingState {
    fn new() -> Self {
        Self {
            blocks: BTreeMap::new(),
            next_tool_call_index: 0,
        }
    }

    fn register_tool_use_block(&mut self, block_index: u32) -> StreamTranslationResult<u32> {
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

    fn register_text_block(&mut self, block_index: u32) -> StreamTranslationResult<()> {
        self.register_block(block_index, StreamBlock::Text)
    }

    fn register_thinking_block(&mut self, block_index: u32) -> StreamTranslationResult<()> {
        self.register_block(block_index, StreamBlock::Thinking)
    }

    fn register_ignored_block(&mut self, block_index: u32) -> StreamTranslationResult<()> {
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

    fn require_block(
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

    fn require_reasoning_signature_block(&self, block_index: u32) -> StreamTranslationResult<()> {
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

    fn get_tool_call_index(&self, block_index: u32) -> StreamTranslationResult<u32> {
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

    fn stop_block(&mut self, block_index: u32) -> StreamTranslationResult<()> {
        self.blocks.remove(&block_index).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Anthropic stream emitted content_block_stop for unopened content block index {block_index}"
            ))
        })?;
        Ok(())
    }
}

#[derive(Debug)]
enum ChatStreamOutput {
    Chunk(CreateChatCompletionStreamResponse),
    DoneSentinel,
}

impl ChatStreamOutput {
    fn encode(self) -> StreamTranslationResult<Bytes> {
        match self {
            Self::Chunk(payload) => Ok(encode_sse_json("message", &payload)?),
            Self::DoneSentinel => Ok(done_sentinel_bytes()),
        }
    }
}

impl StreamingEventTranslator for ChatCompletionStreamTranslator {
    fn translate_event(&mut self, event: SseEvent) -> StreamTranslationResult<Vec<Bytes>> {
        let parsed = self.lifecycle.parse_allowed_stream_event(event)?;
        let mut chunks = Vec::new();

        match parsed {
            MessageStreamEvent::MessageStart(event) => {
                let delta: ChatCompletionStreamResponseDelta = (&event).into();
                let identity = StreamIdentity::new(
                    format!("chatcmpl_{}", event.message.id),
                    event.message.model,
                );
                self.lifecycle
                    .begin_message_stream(identity.clone(), StreamingState::new())?;
                chunks.push(ChatStreamOutput::Chunk(chat_choice_chunk(
                    &identity, delta, None,
                )));
            }
            MessageStreamEvent::Ping(_) => {}

            MessageStreamEvent::ContentBlockStart(event) => {
                let index = event.index;
                match event.content_block {
                    ContentBlock::Text(block) => {
                        self.lifecycle
                            .streaming_state_mut()?
                            .register_text_block(index)?;
                        if !block.text.is_empty() {
                            self.lifecycle.streaming_phase_mut()?.mark_text();
                            let identity = self.lifecycle.stream_identity()?;
                            chunks.push(ChatStreamOutput::Chunk(chat_choice_chunk(
                                identity,
                                block.into(),
                                None,
                            )));
                        }
                    }
                    ContentBlock::ToolUse(block) => {
                        let tool_call_index = {
                            let state = self.lifecycle.streaming_state_mut()?;
                            state.register_tool_use_block(index)?
                        };
                        self.lifecycle.streaming_phase_mut()?.mark_tool_use();
                        let identity = self.lifecycle.stream_identity()?;
                        chunks.push(ChatStreamOutput::Chunk(chat_choice_chunk(
                            identity,
                            ToolStartDelta {
                                index: tool_call_index,
                                block,
                            }
                            .into(),
                            None,
                        )));
                    }

                    ContentBlock::Thinking(block) => {
                        self.lifecycle
                            .streaming_state_mut()?
                            .register_thinking_block(index)?;
                        if !block.thinking.is_empty() {
                            self.lifecycle.streaming_phase_mut()?.mark_reasoning();
                            let identity = self.lifecycle.stream_identity()?;
                            chunks.push(ChatStreamOutput::Chunk(chat_choice_chunk(
                                identity,
                                block.into(),
                                None,
                            )));
                        }
                    }
                    ContentBlock::RedactedThinking(_) => {
                        tracing::trace!(
                            block_index = index,
                            "skipping Anthropic redacted_thinking block with no Chat-representable field"
                        );
                        self.lifecycle
                            .streaming_state_mut()?
                            .register_ignored_block(index)?;
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
                            "Anthropic stream emitted content_block_start that Chat Completions cannot represent"
                                .to_string(),
                        ));
                    }
                }
            }
            MessageStreamEvent::ContentBlockDelta(event) => match event.delta {
                ContentBlockDelta::TextDelta(delta) => {
                    self.lifecycle.streaming_state()?.require_block(
                        event.index,
                        StreamBlock::Text,
                        "text_delta",
                    )?;
                    if !delta.text.is_empty() {
                        self.lifecycle.streaming_phase_mut()?.mark_text();
                        let identity = self.lifecycle.stream_identity()?;
                        chunks.push(ChatStreamOutput::Chunk(chat_choice_chunk(
                            identity,
                            delta.into(),
                            None,
                        )));
                    }
                }
                ContentBlockDelta::InputJsonDelta(delta) => {
                    let tool_call_index = self
                        .lifecycle
                        .streaming_state()?
                        .get_tool_call_index(event.index)?;

                    let identity = self.lifecycle.stream_identity()?;
                    chunks.push(ChatStreamOutput::Chunk(chat_choice_chunk(
                        identity,
                        ToolArgumentsDelta {
                            index: tool_call_index,
                            delta,
                        }
                        .into(),
                        None,
                    )));
                }

                ContentBlockDelta::ThinkingDelta(delta) => {
                    self.lifecycle.streaming_state()?.require_block(
                        event.index,
                        StreamBlock::Thinking,
                        "thinking_delta",
                    )?;
                    if !delta.thinking.is_empty() {
                        self.lifecycle.streaming_phase_mut()?.mark_reasoning();
                        let identity = self.lifecycle.stream_identity()?;
                        chunks.push(ChatStreamOutput::Chunk(chat_choice_chunk(
                            identity,
                            delta.into(),
                            None,
                        )));
                    }
                }
                ContentBlockDelta::SignatureDelta(_) => {
                    self.lifecycle
                        .streaming_state()?
                        .require_reasoning_signature_block(event.index)?;
                }

                ContentBlockDelta::CitationsDelta(_) => {
                    return Err(StreamTranslationError::Semantic(
                        "Anthropic stream emitted content_block_delta that Chat Completions cannot represent"
                            .to_string(),
                    ));
                }
            },
            MessageStreamEvent::MessageDelta(event) => {
                let Some(stop_reason) = event.delta.stop_reason else {
                    return Err(StreamTranslationError::Semantic(
                        "Anthropic stream emitted message_delta without stop_reason".to_string(),
                    ));
                };

                let mut phase = self.lifecycle.take_streaming_phase()?;
                let emitted_text = phase.emitted_text();
                let emitted_representable_content = phase.emitted_any();
                let terminal_delta = chat_terminal_delta(event.delta, emitted_text);
                let identity = self.lifecycle.stream_identity()?.clone();
                let finish_reason = stop_reason.into();

                match terminal_delta {
                    ChatTerminalDelta::Refusal(refusal) => {
                        phase.mark_refusal();
                        chunks.push(ChatStreamOutput::Chunk(chat_choice_chunk(
                            &identity,
                            ChatCompletionStreamResponseDelta {
                                refusal: Some(refusal),
                                ..Default::default()
                            },
                            None,
                        )));
                        chunks.push(ChatStreamOutput::Chunk(chat_choice_chunk(
                            &identity,
                            ChatCompletionStreamResponseDelta::default(),
                            Some(finish_reason),
                        )));
                    }
                    ChatTerminalDelta::Empty => {
                        if !emitted_representable_content {
                            return Err(StreamTranslationError::Semantic(
                                "Anthropic stream completed without Chat-representable content, thinking, refusal, or tool_use blocks"
                                    .to_string(),
                            ));
                        }
                        chunks.push(ChatStreamOutput::Chunk(chat_choice_chunk(
                            &identity,
                            ChatCompletionStreamResponseDelta::default(),
                            Some(finish_reason),
                        )));
                    }
                }

                // Chat streaming usage is a response-level update. Keep it
                // in a separate `choices: []` chunk, matching OpenAI's
                // `stream_options.include_usage` shape, instead of merging it
                // into a content or terminal choice chunk.
                chunks.push(ChatStreamOutput::Chunk(chat_usage_chunk(
                    &identity,
                    event.usage.into(),
                )));

                self.lifecycle.receive_terminal_delta(phase);
            }
            MessageStreamEvent::MessageStop(_) => {
                let _phase = self.lifecycle.take_terminal_phase()?;
                self.lifecycle.stop();
                chunks.push(ChatStreamOutput::DoneSentinel);
            }
            MessageStreamEvent::ContentBlockStop(event) => {
                self.lifecycle
                    .streaming_state_mut()?
                    .stop_block(event.index)?;
            }
        }

        chunks
            .into_iter()
            .map(ChatStreamOutput::encode)
            .collect::<StreamTranslationResult<Vec<_>>>()
    }

    fn finish_stream(&mut self, end: SseStreamEnd) -> StreamTranslationResult<Vec<Bytes>> {
        if self.lifecycle.is_stopped() {
            return Ok(Vec::new());
        }

        Err(self.lifecycle.unexpected_stream_end_error(end))
    }
}

fn chat_choice_chunk(
    identity: &StreamIdentity,
    delta: ChatCompletionStreamResponseDelta,
    finish_reason: Option<FinishReason>,
) -> CreateChatCompletionStreamResponse {
    CreateChatCompletionStreamResponse {
        id: identity.id().to_string(),
        choices: vec![ChatChoiceStream {
            index: 0,
            delta,
            finish_reason,
            logprobs: None,
        }],
        created: 0,
        model: identity.model().to_string(),
        service_tier: None,
        object: "chat.completion.chunk".to_string(),
        usage: None,
    }
}

fn chat_usage_chunk(
    identity: &StreamIdentity,
    usage: CompletionUsage,
) -> CreateChatCompletionStreamResponse {
    CreateChatCompletionStreamResponse {
        id: identity.id().to_string(),
        choices: Vec::new(),
        created: 0,
        model: identity.model().to_string(),
        service_tier: None,
        object: "chat.completion.chunk".to_string(),
        usage: Some(usage),
    }
}

enum ChatTerminalDelta {
    Refusal(String),
    Empty,
}

fn chat_terminal_delta(delta: MessageDelta, emitted_text: bool) -> ChatTerminalDelta {
    // MessageDelta.stop_reason is converted by the caller into Chat's
    // choice-level `finish_reason`; Chat stream deltas have no field for
    // Anthropic `container` or `stop_sequence`.
    //
    // Non-streaming refusal conversion can move final text into
    // `message.refusal` and leave `message.content` empty. Streaming cannot
    // retract text deltas that were already sent without buffering the whole
    // response, so only emit `delta.refusal` when no text content has been
    // streamed yet.
    if emitted_text || !matches!(delta.stop_reason, Some(StopReason::Refusal)) {
        return ChatTerminalDelta::Empty;
    }

    let Some(stop_details) = delta.stop_details else {
        return ChatTerminalDelta::Empty;
    };

    let Some(explanation) = stop_details.explanation else {
        return ChatTerminalDelta::Empty;
    };

    ChatTerminalDelta::Refusal(explanation)
}

struct ToolStartDelta {
    index: u32,
    block: ToolUseBlock,
}

struct ToolArgumentsDelta {
    index: u32,
    delta: InputJsonDelta,
}

impl From<&MessageStartEvent> for ChatCompletionStreamResponseDelta {
    fn from(_event: &MessageStartEvent) -> Self {
        // Anthropic `message_start` opens the assistant message envelope.
        // Chat Completions streams represent that as the initial role delta.
        Self {
            content: None,
            tool_calls: None,
            role: Some(Role::Assistant),
            refusal: None,
            reasoning_content: None,
        }
    }
}

impl From<ToolStartDelta> for ChatCompletionStreamResponseDelta {
    fn from(delta: ToolStartDelta) -> Self {
        Self {
            content: None,
            tool_calls: Some(vec![ChatCompletionMessageToolCallChunk {
                index: delta.index,
                id: Some(delta.block.id),
                r#type: Some(FunctionType::Function),
                function: Some(FunctionCallStream {
                    name: Some(delta.block.name),
                    // Start the Chat arguments stream with an empty string. `None`
                    // would serialize as JSON null in the local wire model, while
                    // OpenAI-compatible tool argument deltas are string fragments.
                    arguments: Some(String::new()),
                }),
            }]),
            role: None,
            refusal: None,
            reasoning_content: None,
        }
    }
}

impl From<ToolArgumentsDelta> for ChatCompletionStreamResponseDelta {
    fn from(delta: ToolArgumentsDelta) -> Self {
        Self {
            content: None,
            tool_calls: Some(vec![ChatCompletionMessageToolCallChunk {
                index: delta.index,
                id: None,
                r#type: None,
                function: Some(FunctionCallStream {
                    name: None,
                    arguments: Some(delta.delta.partial_json),
                }),
            }]),
            role: None,
            refusal: None,
            reasoning_content: None,
        }
    }
}

#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
