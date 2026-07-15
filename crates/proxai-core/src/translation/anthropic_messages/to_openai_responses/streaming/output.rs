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
use crate::protocol::openai_responses::{OutputItem, ResponseStreamEvent};
use crate::translation::TranslationScope;
use crate::translation::anthropic_messages::continuation::{Continuation, ContinuationEnvelope};
use crate::translation::openai_responses::outbound::{
    completed_function_call_item_with_id, output_text_done, reasoning_item, reasoning_text_done,
    redacted_reasoning_item, text_message_item, tool_arguments_done,
};
use crate::translation::stream::StreamTranslationResult;

use super::super::citations::text_block_annotations;
use super::state::StreamBlock;

/// Build the finalized `OutputItem` and any per-content "done" events for a
/// content block that has just received `content_block_stop`.
///
/// Returns `(item, content_done_events)`:
/// - `item` is appended to `StreamingState::output_items` by the caller and
///   also drives the protocol-mandated `response.output_item.done` event,
///   which the caller emits separately via `output_item_done`.
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
    scope: &TranslationScope,
) -> StreamTranslationResult<(OutputItem, Vec<ResponseStreamEvent>)> {
    Ok(match block {
        StreamBlock::Text {
            item_id,
            text,
            citations,
        } => {
            let done =
                output_text_done(sequence_number, item_id.clone(), output_index, text.clone());
            // Translate Anthropic citations to Responses URL annotations
            // using the cumulative character offset of all previous text
            // items, mirroring the non-streaming conversion in response.rs.
            let synthetic_block = TextBlock {
                text: text.clone(),
                citations: citations.into(),
            };
            let annotations = text_block_annotations(&synthetic_block, *text_char_offset, scope);
            *text_char_offset = text_char_offset.saturating_add(text.chars().count());
            let item = text_message_item(item_id, text, annotations);
            (item, vec![done])
        }
        StreamBlock::Thinking {
            item_id,
            text,
            signature,
        } => {
            let done =
                reasoning_text_done(sequence_number, item_id.clone(), output_index, text.clone());
            let mut item = reasoning_item(item_id, text.clone());
            if let OutputItem::Reasoning(item) = &mut item {
                item.encrypted_content = Some(
                    ContinuationEnvelope::from(vec![Continuation::Thinking {
                        thinking: text,
                        signature,
                    }])
                    .encode()?,
                )
                .into();
            }
            (item, vec![done])
        }
        StreamBlock::RedactedThinking { item_id, data } => {
            let mut item = redacted_reasoning_item(item_id, data.clone());
            if let OutputItem::Reasoning(item) = &mut item {
                item.encrypted_content = Some(
                    ContinuationEnvelope::from(vec![Continuation::RedactedThinking { data }])
                        .encode()?,
                )
                .into();
            }
            (item, Vec::new())
        }
        StreamBlock::ToolUse {
            item_id,
            name,
            arguments,
        } => {
            let done = tool_arguments_done(
                sequence_number,
                item_id.clone(),
                output_index,
                name.clone(),
                arguments.clone(),
            );
            let item = completed_function_call_item_with_id(item_id, name, arguments);
            (item, vec![done])
        }
    })
}
