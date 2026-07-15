//! Per-message accumulation state for
//! `openai_chat_completions -> openai_responses` streaming translation.
//!
//! Tracks one in-flight text item (Chat Completions streaming only produces
//! a single assistant message — multiple choices are rejected by the
//! translator) and any number of parallel tool-call items.
//!
//! Terminal event construction combines this state with pair-local output
//! helpers and shared `openai_responses::outbound` event builders.

use std::collections::BTreeMap;

use crate::protocol::openai::chat_completions::{
    ChatCompletionMessageToolCallChunk, CompletionUsage, CreateChatCompletionStreamResponse,
    FinishReason,
};
use crate::protocol::openai_responses::{
    IncompleteDetails, InputTokenDetails, OutputItem, OutputTokenDetails, Response, ResponseObject,
    ResponseStreamEvent, ResponseUsage, Status, ToolChoiceOptions, ToolChoiceParam,
};
use crate::translation::stream::{StreamTranslationError, StreamTranslationResult};

use super::super::types::{
    incomplete_details_from_finish_reason, responses_status_from_chat_finish_reason,
};
use super::types::{StreamTextItem, StreamToolItem};
use crate::translation::openai_responses::outbound::{
    completed_function_call_item_with_id, in_progress_function_call_item, in_progress_message_item,
    in_progress_reasoning_item, output_item_added, output_item_done, output_text_done,
    reasoning_item, reasoning_text_done, refusal_done, refusal_message_item, response_created,
    response_id, response_terminal, text_message_item, tool_arguments_done,
};

#[derive(Debug)]
pub(super) struct StreamingState {
    pub(super) sequence_number: u64,
    pub(super) response_id: String,
    pub(super) model: String,
    pub(super) created_at: f64,
    next_output_index: u32,
    pub(super) text_item: Option<StreamTextItem>,
    pub(super) refusal_item: Option<StreamTextItem>,
    pub(super) reasoning_item: Option<StreamTextItem>,
    pub(super) tool_items: BTreeMap<u32, StreamToolItem>,
    pub(super) output_items: Vec<OutputItem>,
    pub(super) usage: Option<CompletionUsage>,
}

#[derive(Debug)]
enum PendingOutput {
    Text(StreamTextItem),
    Refusal(StreamTextItem),
    Reasoning(StreamTextItem),
    Tool(StreamToolItem),
}

impl PendingOutput {
    fn output_index(&self) -> u32 {
        match self {
            Self::Text(item) | Self::Refusal(item) | Self::Reasoning(item) => item.output_index,
            Self::Tool(item) => item.output_index,
        }
    }
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
        let created_at = chunk.created as f64;

        Ok(Self {
            sequence_number: 0,
            response_id,
            model,
            created_at,
            next_output_index: 0,
            text_item: None,
            refusal_item: None,
            reasoning_item: None,
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
        response_created(
            sequence_number,
            self.response_snapshot(Status::InProgress, None),
        )
    }

    pub(super) fn response_terminal_event(
        &mut self,
        finish_reason: FinishReason,
    ) -> ResponseStreamEvent {
        let status = responses_status_from_chat_finish_reason(finish_reason);
        let incomplete_details = incomplete_details_from_finish_reason(Some(finish_reason));
        let sequence_number = self.next_sequence_number();
        response_terminal(
            sequence_number,
            self.response_snapshot(status, incomplete_details),
            status,
        )
    }

    /// Build the terminal event sequence for a completed stream: per-item
    /// `done` events (text / tool), each followed by `output_item.done`,
    /// and finally `response.completed`.
    ///
    /// Finalized items are pushed into `self.output_items` so the terminal
    /// `response.completed` snapshot's `output` field reflects what the
    /// stream actually produced.
    pub(super) fn finish_stream(
        &mut self,
        finish_reason: FinishReason,
    ) -> Vec<ResponseStreamEvent> {
        let mut events = Vec::new();
        let mut pending = Vec::new();
        if let Some(item) = self.text_item.take() {
            pending.push(PendingOutput::Text(item));
        }
        if let Some(item) = self.refusal_item.take() {
            pending.push(PendingOutput::Refusal(item));
        }
        if let Some(item) = self.reasoning_item.take() {
            pending.push(PendingOutput::Reasoning(item));
        }
        pending.extend(
            std::mem::take(&mut self.tool_items)
                .into_values()
                .map(PendingOutput::Tool),
        );
        pending.sort_by_key(PendingOutput::output_index);

        for output in pending {
            let output_index = output.output_index();
            match output {
                PendingOutput::Text(item) => {
                    let sequence_number = self.next_sequence_number();
                    events.push(output_text_done(
                        sequence_number,
                        item.item_id.clone(),
                        output_index,
                        item.text.clone(),
                    ));
                    let output_item = text_message_item(item.item_id, item.text, Vec::new());
                    self.finish_output_item(output_index, output_item, &mut events);
                }
                PendingOutput::Refusal(item) => {
                    let sequence_number = self.next_sequence_number();
                    events.push(refusal_done(
                        sequence_number,
                        item.item_id.clone(),
                        output_index,
                        item.text.clone(),
                    ));
                    let output_item = refusal_message_item(item.item_id, item.text);
                    self.finish_output_item(output_index, output_item, &mut events);
                }
                PendingOutput::Reasoning(item) => {
                    let sequence_number = self.next_sequence_number();
                    events.push(reasoning_text_done(
                        sequence_number,
                        item.item_id.clone(),
                        output_index,
                        item.text.clone(),
                    ));
                    let output_item = reasoning_item(item.item_id, item.text);
                    self.finish_output_item(output_index, output_item, &mut events);
                }
                PendingOutput::Tool(item) => {
                    let sequence_number = self.next_sequence_number();
                    events.push(tool_arguments_done(
                        sequence_number,
                        item.item_id.clone(),
                        output_index,
                        item.name.clone(),
                        item.arguments.clone(),
                    ));
                    let output_item = completed_function_call_item_with_id(
                        item.item_id,
                        item.name,
                        item.arguments,
                    );
                    self.finish_output_item(output_index, output_item, &mut events);
                }
            }
        }
        events.push(self.response_terminal_event(finish_reason));
        events
    }

    pub(super) fn response_snapshot(
        &self,
        status: Status,
        incomplete_details: Option<IncompleteDetails>,
    ) -> Response {
        let usage = self.usage.as_ref();
        let input_tokens = usage.map(|usage| usage.prompt_tokens).unwrap_or_default();
        let output_tokens = usage
            .map(|usage| usage.completion_tokens)
            .unwrap_or_default();
        let total_tokens = usage
            .map(|usage| usage.total_tokens)
            .unwrap_or_else(|| input_tokens.saturating_add(output_tokens));

        Response {
            background: None.into(),
            conversation: None.into(),
            created_at: self.created_at,
            completed_at: None.into(),
            error: None.into(),
            id: self.response_id.clone(),
            incomplete_details: incomplete_details.into(),
            instructions: None.into(),
            max_output_tokens: None.into(),
            max_tool_calls: None.into(),
            metadata: None.into(),
            model: self.model.clone(),
            object: ResponseObject::Response,
            output: self.output_items.clone(),
            output_text: None.into(),
            parallel_tool_calls: false,
            previous_response_id: None.into(),
            prompt: None.into(),
            prompt_cache_key: None,
            prompt_cache_retention: None.into(),
            reasoning: None.into(),
            safety_identifier: None,
            service_tier: None.into(),
            status: Some(status),
            temperature: None.into(),
            text: None,
            tool_choice: ToolChoiceParam::Mode(ToolChoiceOptions::Auto),
            tools: Vec::new(),
            top_logprobs: None.into(),
            top_p: None.into(),
            truncation: None.into(),
            user: None,
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
    pub(super) fn ensure_text_item(
        &mut self,
    ) -> StreamTranslationResult<Option<ResponseStreamEvent>> {
        if self.refusal_item.is_some() {
            return Err(StreamTranslationError::Semantic(
                "Chat stream contains both content and refusal deltas; Responses keeps refusal separate from normal output text"
                    .to_string(),
            ));
        }
        if self.text_item.is_some() {
            return Ok(None);
        }
        let output_index = self.allocate_output_index();
        let item_id = format!("msg_{}", self.response_id);
        let sequence_number = self.next_sequence_number();
        let event = output_item_added(
            sequence_number,
            output_index,
            in_progress_message_item(item_id.clone()),
        );
        self.text_item = Some(StreamTextItem::new(output_index, item_id));
        Ok(Some(event))
    }

    pub(super) fn ensure_refusal_item(
        &mut self,
    ) -> StreamTranslationResult<Option<ResponseStreamEvent>> {
        if self.text_item.is_some() {
            return Err(StreamTranslationError::Semantic(
                "Chat stream contains both content and refusal deltas; Responses keeps refusal separate from normal output text"
                    .to_string(),
            ));
        }
        if self.refusal_item.is_some() {
            return Ok(None);
        }
        let output_index = self.allocate_output_index();
        let item_id = format!("msg_{}_refusal", self.response_id);
        let sequence_number = self.next_sequence_number();
        let event = output_item_added(
            sequence_number,
            output_index,
            in_progress_message_item(item_id.clone()),
        );
        self.refusal_item = Some(StreamTextItem::new(output_index, item_id));
        Ok(Some(event))
    }

    pub(super) fn ensure_reasoning_item(&mut self) -> Option<ResponseStreamEvent> {
        if self.reasoning_item.is_some() {
            return None;
        }
        let output_index = self.allocate_output_index();
        let item_id = format!("rs_{}", self.response_id);
        let sequence_number = self.next_sequence_number();
        let event = output_item_added(
            sequence_number,
            output_index,
            in_progress_reasoning_item(item_id.clone()),
        );
        self.reasoning_item = Some(StreamTextItem::new(output_index, item_id));
        Some(event)
    }

    pub(super) fn append_text_delta(&mut self, delta: &str) -> Option<(String, u32, u64)> {
        let item = self.text_item.as_mut()?;
        item.append(delta);
        let item_id = item.item_id.clone();
        let output_index = item.output_index;
        Some((item_id, output_index, self.next_sequence_number()))
    }

    pub(super) fn append_refusal_delta(&mut self, delta: &str) -> Option<(String, u32, u64)> {
        let item = self.refusal_item.as_mut()?;
        item.append(delta);
        let item_id = item.item_id.clone();
        let output_index = item.output_index;
        Some((item_id, output_index, self.next_sequence_number()))
    }

    pub(super) fn append_reasoning_delta(&mut self, delta: &str) -> Option<(String, u32, u64)> {
        let item = self.reasoning_item.as_mut()?;
        item.append(delta);
        let item_id = item.item_id.clone();
        let output_index = item.output_index;
        Some((item_id, output_index, self.next_sequence_number()))
    }

    pub(super) fn set_tool_name(&mut self, index: u32, name: &str) {
        if let Some(item) = self.tool_items.get_mut(&index) {
            item.set_name(name);
        }
    }

    pub(super) fn append_tool_arguments_delta(
        &mut self,
        index: u32,
        delta: &str,
    ) -> Option<(String, u32, u64)> {
        let item = self.tool_items.get_mut(&index)?;
        item.append_arguments(delta);
        let item_id = item.item_id.clone();
        let output_index = item.output_index;
        Some((item_id, output_index, self.next_sequence_number()))
    }

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
        let output_index = self.allocate_output_index();
        let sequence_number = self.next_sequence_number();
        let event = output_item_added(
            sequence_number,
            output_index,
            in_progress_function_call_item(item_id.clone(), name.clone()),
        );
        self.tool_items
            .insert(index, StreamToolItem::new(output_index, item_id, name));
        Ok(Some(event))
    }

    fn allocate_output_index(&mut self) -> u32 {
        let index = self.next_output_index;
        self.next_output_index = self.next_output_index.saturating_add(1);
        index
    }

    fn finish_output_item(
        &mut self,
        output_index: u32,
        output_item: OutputItem,
        events: &mut Vec<ResponseStreamEvent>,
    ) {
        let sequence_number = self.next_sequence_number();
        events.push(output_item_done(
            sequence_number,
            output_index,
            output_item.clone(),
        ));
        self.output_items.push(output_item);
    }
}
