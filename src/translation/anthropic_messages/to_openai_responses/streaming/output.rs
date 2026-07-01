//! Output builders for
//! `anthropic_messages -> openai_responses` streaming translation.
//!
//! These constructors take finalized per-block state (the values returned by
//! `state::StreamingState::stop_block`) and assemble the matching Responses
//! terminal events plus the finalized `OutputItem`. They hold no streaming
//! state of their own; `text_char_offset` is threaded in as a mutable
//! reference because citations-to-annotations translation depends on the
//! cumulative text offset across the whole stream.

use crate::protocol::anthropic::messages::TextBlock;
use crate::protocol::openai_responses::{
    AssistantRole, FunctionToolCall, OutputItem, OutputMessage, OutputMessageContent, OutputStatus,
    OutputTextContent, ReasoningItem, ReasoningItemContent, ReasoningTextContent, Response,
    ResponseCompletedEvent, ResponseCreatedEvent, ResponseFunctionCallArgumentsDeltaEvent,
    ResponseFunctionCallArgumentsDoneEvent, ResponseIncompleteEvent, ResponseOutputItemAddedEvent,
    ResponseOutputItemDoneEvent, ResponseReasoningTextDeltaEvent, ResponseReasoningTextDoneEvent,
    ResponseStreamEvent, ResponseTextDeltaEvent, ResponseTextDoneEvent, Status,
};
use crate::translation::streaming::StreamTranslationResult;

use super::super::citations::text_block_annotations;
use super::state::StreamBlock;

/// Build the finalized `OutputItem` and any per-content "done" events for a
/// content block that has just received `content_block_stop`.
///
/// Returns `(item, content_done_events)`:
/// - `item` is appended to `StreamingState::output_items` by the caller and
///   also drives the protocol-mandated `response.output_item.done` event,
///   which the caller emits separately via `output_item_done_event`.
/// - `content_done_events` are the variant-specific content-close events
///   (`response.output_text.done`, `response.reasoning_text.done`,
///   `response.function_call_arguments.done`). Redacted thinking emits none
///   because its opaque payload never had a streamed text/arguments delta
///   sequence to close.
///
/// `sequence_number` is the value the caller already advanced its counter
/// to for this block's done event. The caller owns sequence-number advance;
/// this helper only consumes the value. (Redacted thinking does not emit a
/// done event, so the caller-advanced number goes unused for that variant —
/// sequence numbers only need to be monotonic, so a skipped value is fine.)
///
/// `text_char_offset` is read and updated only for the `Text` variant, where
/// Anthropic citations must be translated to Responses URL annotations using
/// character indices relative to the full text output so far.
pub(super) fn finalize_block(
    block: StreamBlock,
    output_index: u32,
    sequence_number: u64,
    text_char_offset: &mut usize,
) -> StreamTranslationResult<(OutputItem, Vec<ResponseStreamEvent>)> {
    Ok(match block {
        StreamBlock::Text {
            item_id,
            text,
            citations,
        } => {
            let done = ResponseStreamEvent::ResponseOutputTextDone(ResponseTextDoneEvent {
                sequence_number,
                item_id: item_id.clone(),
                output_index,
                content_index: 0,
                text: text.clone(),
                logprobs: None,
            });
            // Translate Anthropic citations to Responses URL annotations
            // using the cumulative character offset of all previous text
            // items, mirroring the non-streaming conversion in response.rs.
            let synthetic_block = TextBlock {
                text: text.clone(),
                citations,
            };
            let annotations = text_block_annotations(&synthetic_block, *text_char_offset);
            *text_char_offset = text_char_offset.saturating_add(text.chars().count());
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
            let done =
                ResponseStreamEvent::ResponseReasoningTextDone(ResponseReasoningTextDoneEvent {
                    sequence_number,
                    item_id: item_id.clone(),
                    output_index,
                    content_index: 0,
                    text: text.clone(),
                });
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
            // Redacted thinking has no streamed text deltas; only the
            // lifecycle close events are emitted. The `encrypted_content`
            // field carries the opaque payload that non-streaming
            // translation also surfaces.
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
            let done = ResponseStreamEvent::ResponseFunctionCallArgumentsDone(
                ResponseFunctionCallArgumentsDoneEvent {
                    sequence_number,
                    item_id: item_id.clone(),
                    output_index,
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
    })
}

/// Build the protocol-mandated `response.output_item.done` event that closes
/// the lifecycle opened by `response.output_item.added`, regardless of which
/// block variant produced the item.
pub(super) fn output_item_done_event(
    output_index: u32,
    item: OutputItem,
    sequence_number: u64,
) -> ResponseStreamEvent {
    ResponseStreamEvent::ResponseOutputItemDone(ResponseOutputItemDoneEvent {
        sequence_number,
        output_index,
        item,
    })
}

// ---------------------------------------------------------------------
// Initial OutputItem variants (status: InProgress) for content_block_start.
// ---------------------------------------------------------------------

/// Empty assistant message shell opened by `content_block_start` of a text
/// block. Content is filled in by subsequent text deltas.
pub(super) fn message_item_initial(item_id: String) -> OutputItem {
    OutputItem::Message(OutputMessage {
        id: item_id,
        role: AssistantRole::Assistant,
        status: OutputStatus::InProgress,
        content: Vec::new(),
        phase: None,
    })
}

/// Reasoning item opened by `content_block_start` of a thinking block. Text
/// content is filled in by subsequent thinking deltas.
pub(super) fn reasoning_item_initial(item_id: String) -> OutputItem {
    OutputItem::Reasoning(ReasoningItem {
        id: Some(item_id),
        summary: Vec::new(),
        content: Some(Vec::new()),
        encrypted_content: None,
        status: Some(OutputStatus::InProgress),
    })
}

/// Reasoning item opened by `content_block_start` of a redacted-thinking
/// block. No streamed text deltas; the opaque `encrypted_content` arrives
/// with `content_block_stop` and is attached by `finalize_block`.
pub(super) fn redacted_reasoning_item_initial(item_id: String) -> OutputItem {
    OutputItem::Reasoning(ReasoningItem {
        id: Some(item_id),
        summary: Vec::new(),
        content: None,
        // Placeholder; the real data arrives with content_block_stop.
        encrypted_content: None,
        status: Some(OutputStatus::InProgress),
    })
}

/// Function-call item opened by `content_block_start` of a tool_use block.
/// Arguments are filled in by subsequent `input_json_delta` events.
pub(super) fn tool_use_item_initial(item_id: String, name: String) -> OutputItem {
    OutputItem::FunctionCall(FunctionToolCall {
        id: Some(item_id.clone()),
        call_id: item_id,
        name,
        arguments: String::new(),
        status: Some(OutputStatus::InProgress),
        namespace: None,
    })
}

// ---------------------------------------------------------------------
// ResponseStreamEvent constructors for lifecycle / delta events.
// ---------------------------------------------------------------------

pub(super) fn response_created(sequence_number: u64, response: Response) -> ResponseStreamEvent {
    ResponseStreamEvent::ResponseCreated(ResponseCreatedEvent {
        sequence_number,
        response,
    })
}

/// Terminal snapshot event. `Incomplete` and `Completed` are the two
/// representable terminal statuses for an Anthropic stream; the variant is
/// selected by `status`.
pub(super) fn response_terminal(
    sequence_number: u64,
    response: Response,
    status: Status,
) -> ResponseStreamEvent {
    match status {
        Status::Incomplete => ResponseStreamEvent::ResponseIncomplete(ResponseIncompleteEvent {
            sequence_number,
            response,
        }),
        _ => ResponseStreamEvent::ResponseCompleted(ResponseCompletedEvent {
            sequence_number,
            response,
        }),
    }
}

pub(super) fn output_item_added(
    sequence_number: u64,
    output_index: u32,
    item: OutputItem,
) -> ResponseStreamEvent {
    ResponseStreamEvent::ResponseOutputItemAdded(ResponseOutputItemAddedEvent {
        sequence_number,
        output_index,
        item,
    })
}

// The constructors below (`output_text_delta`, `tool_arguments_delta`,
// `reasoning_text_delta`) are mirrored verbatim in
// `openai_chat_completions::to_openai_responses::streaming::output` (the first
// two) because both pairs emit the same Responses streaming event shapes. If a
// third pair translating to Responses appears, extract these into a shared
// module under `protocol::openai_responses` or `translation::streaming`.

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

pub(super) fn reasoning_text_delta(
    sequence_number: u64,
    item_id: String,
    output_index: u32,
    delta: String,
) -> ResponseStreamEvent {
    ResponseStreamEvent::ResponseReasoningTextDelta(ResponseReasoningTextDeltaEvent {
        sequence_number,
        item_id,
        output_index,
        content_index: 0,
        delta,
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
