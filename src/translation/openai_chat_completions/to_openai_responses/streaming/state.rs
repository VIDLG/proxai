//! Per-message accumulation state for
//! `openai_chat_completions -> openai_responses` streaming translation.
//!
//! Tracks one in-flight text item (Chat Completions streaming only produces
//! a single assistant message — multiple choices are rejected by the
//! translator) and any number of parallel tool-call items.
//!
//! Terminal event construction (`response.created`, per-item `done` events,
//! `response.completed`) lives in `output::finalize_completed_stream`; this
//! module owns only the state those events are built from.

use std::collections::BTreeMap;

use crate::protocol::openai::chat_completions::{
    ChatCompletionMessageToolCallChunk, CompletionUsage, CreateChatCompletionStreamResponse,
};
use crate::protocol::openai_responses::{
    AssistantRole, FunctionToolCall, InputTokenDetails, OutputItem, OutputMessage, OutputStatus,
    OutputTokenDetails, Response, ResponseCompletedEvent, ResponseCreatedEvent,
    ResponseOutputItemAddedEvent, ResponseStreamEvent, ResponseUsage, Status,
};
use crate::translation::streaming::{StreamTranslationError, StreamTranslationResult};

use super::super::types::response_id;
use super::output::{
    output_item_done, output_text_done, text_output_item, tool_arguments_done, tool_output_item,
};
use super::types::{StreamTextItem, StreamToolItem};

#[derive(Debug)]
pub(super) struct StreamingState {
    pub(super) sequence_number: u64,
    pub(super) response_id: String,
    pub(super) model: String,
    pub(super) created_at: u64,
    pub(super) text_item: Option<StreamTextItem>,
    pub(super) tool_items: BTreeMap<u32, StreamToolItem>,
    pub(super) output_items: Vec<OutputItem>,
    pub(super) usage: Option<CompletionUsage>,
}

impl StreamingState {
    pub(super) fn new(chunk: &CreateChatCompletionStreamResponse) -> StreamTranslationResult<Self> {
        if chunk.id.is_empty() {
            return Err(StreamTranslationError::Semantic(
                "Chat stream chunk is missing id required for Responses response id".to_string(),
            ));
        }
        if chunk.model.is_empty() {
            return Err(StreamTranslationError::Semantic(
                "Chat stream chunk is missing model required for Responses response snapshot"
                    .to_string(),
            ));
        }
        let response_id = response_id(&chunk.id);
        let model = chunk.model.clone();
        let created_at = chunk.created as u64;

        Ok(Self {
            sequence_number: 0,
            response_id,
            model,
            created_at,
            text_item: None,
            tool_items: BTreeMap::new(),
            output_items: Vec::new(),
            usage: None,
        })
    }

    pub(super) fn next_sequence_number(&mut self) -> u64 {
        let sequence_number = self.sequence_number;
        self.sequence_number += 1;
        sequence_number
    }

    pub(super) fn response_created_event(&mut self) -> ResponseStreamEvent {
        let sequence_number = self.next_sequence_number();
        ResponseStreamEvent::ResponseCreated(ResponseCreatedEvent {
            sequence_number,
            response: self.response_snapshot(Status::InProgress),
        })
    }

    pub(super) fn response_completed_event(&mut self) -> ResponseStreamEvent {
        let sequence_number = self.next_sequence_number();
        ResponseStreamEvent::ResponseCompleted(ResponseCompletedEvent {
            sequence_number,
            response: self.response_snapshot(Status::Completed),
        })
    }

    /// Build the terminal event sequence for a completed stream: per-item
    /// `done` events (text / tool), each followed by `output_item.done`,
    /// and finally `response.completed`.
    ///
    /// Finalized items are pushed into `self.output_items` so the terminal
    /// `response.completed` snapshot's `output` field reflects what the
    /// stream actually produced.
    pub(super) fn finish_completed_stream(&mut self) -> Vec<ResponseStreamEvent> {
        let mut events = Vec::new();

        if let Some(text_item) = self.text_item.take() {
            let sequence_number = self.next_sequence_number();
            events.push(output_text_done(sequence_number, text_item.clone()));

            let output_item = text_output_item(text_item);
            let sequence_number = self.next_sequence_number();
            events.push(output_item_done(sequence_number, 0, output_item.clone()));
            self.output_items.push(output_item);
        }
        for (index, tool_item) in std::mem::take(&mut self.tool_items) {
            let sequence_number = self.next_sequence_number();
            events.push(tool_arguments_done(
                sequence_number,
                tool_item.clone(),
                index,
            ));

            let output_item = tool_output_item(tool_item);
            let sequence_number = self.next_sequence_number();
            events.push(output_item_done(
                sequence_number,
                index,
                output_item.clone(),
            ));
            self.output_items.push(output_item);
        }
        let event = self.response_completed_event();
        events.push(event);
        events
    }

    pub(super) fn response_snapshot(&self, status: Status) -> Response {
        let usage = self.usage.as_ref();
        let input_tokens = usage.map(|usage| usage.prompt_tokens).unwrap_or_default() as u32;
        let output_tokens = usage
            .map(|usage| usage.completion_tokens)
            .unwrap_or_default() as u32;
        let total_tokens = usage
            .map(|usage| usage.total_tokens)
            .unwrap_or_else(|| input_tokens.saturating_add(output_tokens));

        Response {
            background: None,
            billing: None,
            conversation: None,
            created_at: self.created_at,
            completed_at: None,
            error: None,
            id: self.response_id.clone(),
            incomplete_details: None,
            instructions: None,
            max_output_tokens: None,
            metadata: None,
            model: self.model.clone(),
            object: "response".to_string(),
            output: self.output_items.clone(),
            parallel_tool_calls: None,
            previous_response_id: None,
            prompt: None,
            prompt_cache_key: None,
            prompt_cache_retention: None,
            reasoning: None,
            safety_identifier: None,
            service_tier: None,
            status,
            temperature: None,
            text: None,
            tool_choice: None,
            tools: None,
            top_logprobs: None,
            top_p: None,
            truncation: None,
            usage: Some(ResponseUsage {
                input_tokens,
                input_tokens_details: InputTokenDetails { cached_tokens: 0 },
                output_tokens,
                output_tokens_details: OutputTokenDetails {
                    reasoning_tokens: 0,
                },
                total_tokens,
            }),
        }
    }

    /// Ensure a text item slot exists, returning the `response.output_item.added`
    /// event if a new slot was opened.
    ///
    /// Chat Completions streams represent the assistant message text as a
    /// single choice (multiple choices are rejected by the translator); the
    /// Responses side models it as one `OutputItem::Message` whose content
    /// fills in via subsequent text deltas.
    pub(super) fn ensure_text_item(&mut self) -> Option<ResponseStreamEvent> {
        if self.text_item.is_some() {
            return None;
        }
        let item_id = format!("msg_{}", self.response_id);
        let sequence_number = self.next_sequence_number();
        let event = ResponseStreamEvent::ResponseOutputItemAdded(ResponseOutputItemAddedEvent {
            sequence_number,
            output_index: 0,
            item: OutputItem::Message(OutputMessage {
                id: item_id.clone(),
                role: AssistantRole::Assistant,
                status: OutputStatus::InProgress,
                content: Vec::new(),
                phase: None,
            }),
        });
        self.text_item = Some(StreamTextItem::new(item_id));
        Some(event)
    }

    /// Append a text delta to the in-flight text item and return its item id
    /// plus the sequence number for the delta event.
    ///
    /// Caller is responsible for ensuring a text item exists (via
    /// `ensure_text_item`) before calling this.
    pub(super) fn append_text_delta(&mut self, delta: &str) -> Option<(String, u64)> {
        let item = self.text_item.as_mut()?;
        item.append(delta);
        Some((item.item_id.clone(), self.next_sequence_number()))
    }

    /// Update the name of an in-flight tool item.
    pub(super) fn set_tool_name(&mut self, index: u32, name: &str) {
        if let Some(item) = self.tool_items.get_mut(&index) {
            item.set_name(name);
        }
    }

    /// Append arguments to an in-flight tool item and return its item id plus
    /// the sequence number for the delta event. Returns `None` if no tool item
    /// exists for `index`.
    pub(super) fn append_tool_arguments_delta(
        &mut self,
        index: u32,
        delta: &str,
    ) -> Option<(String, u64)> {
        let item = self.tool_items.get_mut(&index)?;
        item.append_arguments(delta);
        Some((item.item_id.clone(), self.next_sequence_number()))
    }

    /// Ensure a tool-call slot exists for the given Chat tool index, returning
    /// the `response.output_item.added` event if a new slot was opened.
    ///
    /// One Chat choice may carry multiple parallel tool calls, each keyed by
    /// its Chat `tool_call.index`; each maps to its own Responses
    /// `OutputItem::FunctionCall`.
    pub(super) fn ensure_tool_item(
        &mut self,
        index: u32,
        tool_call: &ChatCompletionMessageToolCallChunk,
    ) -> StreamTranslationResult<Option<ResponseStreamEvent>> {
        if self.tool_items.contains_key(&index) {
            return Ok(None);
        }
        let item_id = tool_call
            .id
            .as_deref()
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                StreamTranslationError::Semantic(
                    "Chat tool call stream started without a tool call id".to_string(),
                )
            })?
            .to_string();
        let name = tool_call
            .function
            .as_ref()
            .and_then(|function| function.name.as_deref())
            .filter(|name| !name.is_empty())
            .ok_or_else(|| {
                StreamTranslationError::Semantic(
                    "Chat tool call stream started without a function name".to_string(),
                )
            })?
            .to_string();
        let sequence_number = self.next_sequence_number();
        let event = ResponseStreamEvent::ResponseOutputItemAdded(ResponseOutputItemAddedEvent {
            sequence_number,
            output_index: index,
            item: OutputItem::FunctionCall(FunctionToolCall {
                id: Some(item_id.clone()),
                call_id: item_id.clone(),
                name: name.clone(),
                arguments: String::new(),
                status: Some(OutputStatus::InProgress),
                namespace: None,
            }),
        });
        self.tool_items
            .insert(index, StreamToolItem::new(item_id, name));
        Ok(Some(event))
    }
}
