//! Carrier boundary helpers for
//! `openai_chat_completions -> openai_responses` streaming translation.
//!
//! The `response_event` wrapper converts a strongly-typed
//! `ResponseStreamEvent` into a carrier-level `StreamEvent`. Its event_type
//! comes from `ResponseStreamEvent`'s `strum::Display` impl (each variant's
//! `#[strum(serialize = ...)]` yields the SSE event type string, e.g.
//! `"response.created"`); the payload is serialized to JSON by the carrier.
//!
//! The delta constructors below are mirrored verbatim in
//! `anthropic_messages::to_openai_responses::streaming::output` because both
//! pairs emit the same Responses streaming event shapes. If a third pair
//! translating to Responses appears, extract these into a shared module
//! under `protocol::openai_responses` or `translation::streaming`.

use crate::protocol::openai_responses::{
    AssistantRole, FunctionToolCall, OutputItem, OutputMessage, OutputMessageContent, OutputStatus,
    OutputTextContent, ResponseFunctionCallArgumentsDeltaEvent,
    ResponseFunctionCallArgumentsDoneEvent, ResponseOutputItemDoneEvent, ResponseStreamEvent,
    ResponseTextDeltaEvent, ResponseTextDoneEvent,
};
use crate::translation::streaming::{StreamEvent, StreamTranslationResult};

use super::types::{StreamTextItem, StreamToolItem};

pub(super) fn response_event(event: ResponseStreamEvent) -> StreamTranslationResult<StreamEvent> {
    let event_type = event.as_ref().to_string();
    StreamEvent::json(event_type, event)
}

pub(super) fn output_text_delta(
    sequence_number: u64,
    item_id: String,
    output_index: u32,
    delta: String,
) -> ResponseStreamEvent {
    ResponseStreamEvent::ResponseOutputTextDelta(ResponseTextDeltaEvent {
        sequence_number,
        item_id,
        output_index,
        content_index: 0,
        delta,
        logprobs: None,
    })
}

pub(super) fn tool_arguments_delta(
    sequence_number: u64,
    item_id: String,
    output_index: u32,
    delta: String,
) -> ResponseStreamEvent {
    ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(
        ResponseFunctionCallArgumentsDeltaEvent {
            sequence_number,
            item_id,
            output_index,
            delta,
        },
    )
}

// ---------------------------------------------------------------------
// Terminal-event constructors used by `StreamingState::finish_completed_stream`.
// These are pure field assembly; they don't touch streaming state.
// ---------------------------------------------------------------------

pub(super) fn output_text_done(sequence_number: u64, item: StreamTextItem) -> ResponseStreamEvent {
    ResponseStreamEvent::ResponseOutputTextDone(ResponseTextDoneEvent {
        sequence_number,
        item_id: item.item_id,
        output_index: 0,
        content_index: 0,
        text: item.text,
        logprobs: None,
    })
}

pub(super) fn tool_arguments_done(
    sequence_number: u64,
    item: StreamToolItem,
    output_index: u32,
) -> ResponseStreamEvent {
    ResponseStreamEvent::ResponseFunctionCallArgumentsDone(ResponseFunctionCallArgumentsDoneEvent {
        sequence_number,
        item_id: item.item_id,
        output_index,
        name: Some(item.name),
        arguments: item.arguments,
    })
}

pub(super) fn output_item_done(
    sequence_number: u64,
    output_index: u32,
    item: OutputItem,
) -> ResponseStreamEvent {
    ResponseStreamEvent::ResponseOutputItemDone(ResponseOutputItemDoneEvent {
        sequence_number,
        output_index,
        item,
    })
}

pub(super) fn text_output_item(item: StreamTextItem) -> OutputItem {
    OutputItem::Message(OutputMessage {
        id: item.item_id,
        role: AssistantRole::Assistant,
        status: OutputStatus::Completed,
        content: vec![OutputMessageContent::OutputText(OutputTextContent {
            text: item.text,
            annotations: Vec::new(),
            logprobs: None,
        })],
        phase: None,
    })
}

pub(super) fn tool_output_item(item: StreamToolItem) -> OutputItem {
    OutputItem::FunctionCall(FunctionToolCall {
        id: Some(item.item_id.clone()),
        call_id: item.item_id,
        name: item.name,
        arguments: item.arguments,
        status: Some(OutputStatus::Completed),
        namespace: None,
    })
}
