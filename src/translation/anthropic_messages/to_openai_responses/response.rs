use crate::protocol::anthropic::messages::{
    ContentBlock, Message, TextBlock, ToolResultBlock, ToolResultContentParam,
};
use crate::protocol::openai_responses::{
    AssistantRole, FunctionCallOutput, FunctionCallOutputStatusEnum,
    FunctionToolCallOutputResource, OutputItem, OutputMessage, OutputMessageContent, OutputStatus,
    OutputTextContent, ReasoningItem, Response, Status,
};
use crate::translation::{TranslationError, TranslationResult};

use super::citations::text_block_annotations;
use super::ids::OutputItemIdAllocator;
use super::types::{incomplete_details_from_stop_reason, response_id};

impl TryFrom<&Message> for Response {
    type Error = TranslationError;

    fn try_from(message: &Message) -> TranslationResult<Self> {
        let stop_reason = message.stop_reason;
        Ok(Response {
            background: None,
            billing: None,
            conversation: None,
            created_at: 0,
            completed_at: None,
            error: None,
            id: response_id(&message.id),
            incomplete_details: incomplete_details_from_stop_reason(stop_reason),
            instructions: None,
            max_output_tokens: None,
            metadata: None,
            model: message.model.clone(),
            object: "response".to_string(),
            output: translate_output(message)?,
            parallel_tool_calls: None,
            previous_response_id: None,
            prompt: None,
            prompt_cache_key: None,
            prompt_cache_retention: None,
            reasoning: None,
            safety_identifier: None,
            service_tier: message.usage.service_tier.and_then(Into::into),
            status: stop_reason.map(Into::into).unwrap_or(Status::InProgress),
            temperature: None,
            text: None,
            tool_choice: None,
            tools: None,
            top_logprobs: None,
            top_p: None,
            truncation: None,
            usage: Some((&message.usage).into()),
        })
    }
}

fn translate_output(message: &Message) -> TranslationResult<Vec<OutputItem>> {
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
                output.push(text_message_item(ids.message(), block, text_char_offset));
                text_char_offset = text_char_offset.saturating_add(block.text.chars().count());
            }
            ContentBlock::Thinking(block) => {
                let mut item: ReasoningItem = block.into();
                item.id = Some(ids.reasoning());
                output.push(OutputItem::Reasoning(item));
            }
            ContentBlock::RedactedThinking(block) => {
                let mut item: ReasoningItem = block.into();
                item.id = Some(ids.reasoning());
                output.push(OutputItem::Reasoning(item));
            }
            ContentBlock::ToolUse(block) => {
                output.push(OutputItem::FunctionCall(block.try_into()?));
            }
            ContentBlock::ToolResult(block) => {
                output.push(tool_result_output_item(ids.function_call_output(), block)?);
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

fn text_message_item(id: String, block: &TextBlock, base_char_offset: usize) -> OutputItem {
    OutputItem::Message(OutputMessage {
        id,
        role: AssistantRole::Assistant,
        status: OutputStatus::Completed,
        content: vec![OutputMessageContent::OutputText(OutputTextContent {
            text: block.text.clone(),
            annotations: text_block_annotations(block, base_char_offset),
            logprobs: None,
        })],
        phase: None,
    })
}

fn tool_result_output_item(id: String, block: &ToolResultBlock) -> TranslationResult<OutputItem> {
    let output = match block.content.clone() {
        Some(ToolResultContentParam::Text(text)) => FunctionCallOutput::Text(text),
        Some(ToolResultContentParam::Blocks(blocks)) => {
            let parts = blocks
                .into_iter()
                .map(TryInto::try_into)
                .collect::<TranslationResult<Vec<_>>>()?;
            FunctionCallOutput::Content(parts)
        }
        None => FunctionCallOutput::Content(Vec::new()),
    };
    // Anthropic `is_error = true` means the tool execution failed but the
    // result has still been delivered. OpenAI Responses has no `Failed` value
    // in `FunctionCallOutputStatusEnum` (only `InProgress` / `Completed` /
    // `Incomplete`), and `Incomplete` specifically means the output was
    // truncated mid-stream, which is a different semantic. Treat every
    // non-streaming tool result as `Completed`: the error context survives in
    // the `output` payload, which is how OpenAI clients and models normally
    // distinguish successful vs. failed tool executions.
    let status = FunctionCallOutputStatusEnum::Completed;

    Ok(OutputItem::FunctionCallOutput(
        FunctionToolCallOutputResource {
            id,
            call_id: block.tool_use_id.clone(),
            output,
            status,
            created_by: None,
        },
    ))
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
