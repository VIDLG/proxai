use crate::protocol::anthropic::messages::{ContentBlock, Message};
use crate::protocol::openai::responses::{
    OutputItem, Response, ResponseObject, ServiceTier, Status, ToolChoiceOptions, ToolChoiceParam,
};
use crate::translation::anthropic_messages::continuation::{Continuation, ContinuationEnvelope};
use crate::translation::openai_responses::outbound::{response_id, text_message_item};
use crate::translation::{TranslationError, TranslationResult, TranslationScope};

use super::citations::text_block_annotations;
use super::ids::OutputItemIdAllocator;
use super::types::{
    incomplete_details_from_stop_reason, reasoning_item_from_redacted_thinking,
    reasoning_item_from_thinking, responses_status_from_anthropic_stop_reason,
};

pub(super) fn translate_response(
    message: &Message,
    scope: &TranslationScope,
) -> TranslationResult<Response> {
    let stop_reason = message.stop_reason.as_non_null().copied();
    Ok(Response {
        background: None.into(),
        conversation: None.into(),
        created_at: 0.0,
        completed_at: None.into(),
        error: None.into(),
        id: response_id(&message.id),
        incomplete_details: incomplete_details_from_stop_reason(stop_reason).into(),
        instructions: None.into(),
        moderation: None.into(),
        max_output_tokens: None.into(),
        max_tool_calls: None.into(),
        metadata: None.into(),
        model: message.model.clone(),
        object: ResponseObject::Response,
        output: translate_output(message, scope)?,
        output_text: None.into(),
        parallel_tool_calls: false,
        previous_response_id: None.into(),
        prompt: None.into(),
        prompt_cache_key: None,
        prompt_cache_retention: None.into(),
        prompt_cache_options: None,
        reasoning: None.into(),
        safety_identifier: None,
        service_tier: message
            .usage
            .service_tier
            .as_non_null()
            .copied()
            .and_then(Option::<ServiceTier>::from)
            .into(),
        status: Some(
            stop_reason
                .map(responses_status_from_anthropic_stop_reason)
                .unwrap_or(Status::InProgress),
        ),
        temperature: None.into(),
        text: None,
        tool_choice: ToolChoiceParam::Mode(ToolChoiceOptions::Auto),
        tools: Vec::new(),
        top_logprobs: None.into(),
        top_p: None.into(),
        truncation: None.into(),
        user: None,
        usage: Some((&message.usage).into()),
    })
}

fn translate_output(
    message: &Message,
    scope: &TranslationScope,
) -> TranslationResult<Vec<OutputItem>> {
    let mut output = Vec::new();
    let mut ids = OutputItemIdAllocator::new(&message.id);
    // Accumulate the character count of all completed text items so that
    // each subsequent text block's citation annotations use offsets relative
    // to the full text output (matching OpenAI Responses semantics and the
    // streaming translator's `text_char_offset`).
    let mut text_char_offset: usize = 0;

    for block in &message.content {
        match block {
            ContentBlock::Text(block) => {
                // `text_block_annotations` expects the offset of this block's
                // first character within the full text output, i.e. the sum
                // of all preceding text blocks' character counts.
                output.push(text_message_item(
                    ids.message(),
                    &block.text,
                    text_block_annotations(block, text_char_offset, scope),
                ));
                text_char_offset = text_char_offset.saturating_add(block.text.chars().count());
            }
            ContentBlock::Thinking(block) => {
                let mut item = reasoning_item_from_thinking(ids.reasoning(), block);
                item.encrypted_content = Some(
                    ContinuationEnvelope::from(vec![Continuation::Thinking {
                        thinking: block.thinking.clone(),
                        signature: block.signature.clone(),
                    }])
                    .encode()?,
                )
                .into();
                output.push(OutputItem::Reasoning(item));
            }
            ContentBlock::RedactedThinking(block) => {
                let mut item = reasoning_item_from_redacted_thinking(ids.reasoning(), block);
                item.encrypted_content = Some(
                    ContinuationEnvelope::from(vec![Continuation::RedactedThinking {
                        data: block.data.clone(),
                    }])
                    .encode()?,
                )
                .into();
                output.push(OutputItem::Reasoning(item));
            }
            ContentBlock::ToolUse(block) => {
                output.push(OutputItem::FunctionCall(block.try_into()?));
            }
            other => {
                return Err(TranslationError::InvalidPayload(format!(
                    "Anthropic response content block `{}` cannot be translated to OpenAI Responses output item",
                    other.as_ref()
                )));
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
