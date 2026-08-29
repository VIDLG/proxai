//! `openai_responses -> anthropic_messages` non-streaming response translation.
//!
//! Parses an OpenAI Responses `Response` JSON body into Anthropic `Message`
//! shape, including output items (text / tool calls / reasoning) and usage.

use crate::protocol::anthropic::messages::{
    ContentBlock, Message as AnthropicMessage, MessageDeltaUsage, MessageRole, MessageType,
    OutputTokensDetails, Usage,
};
use crate::protocol::openai_responses as responses;
use crate::translation::TranslationScope;
use crate::translation::anthropic_messages::outbound::{
    redacted_thinking_block, text_block, thinking_block, tool_use_block,
};
use crate::translation::openai_responses::stop::infer_response_stop_kind;
use crate::translation::text::parse_json_or_string;

pub(super) fn translate_response_payload(
    response: &responses::Response,
    scope: &TranslationScope,
) -> AnthropicMessage {
    let content = response
        .output
        .iter()
        .flat_map(|item| translate_response_output_item(item, scope))
        .collect::<Vec<_>>();

    let stop_reason = infer_response_stop_kind(response, scope).map(Into::into);

    AnthropicMessage {
        id: response.id.clone(),
        container: None.into(),
        content,
        model: response.model.clone(),
        role: MessageRole::Assistant,
        type_: MessageType::Message,
        stop_details: None.into(),
        stop_reason: stop_reason.into(),
        stop_sequence: None.into(),
        usage: response.usage.as_ref().map(Into::into).unwrap_or_default(),
    }
}

fn translate_response_output_item(
    item: &responses::OutputItem,
    scope: &TranslationScope,
) -> Vec<ContentBlock> {
    match item {
        responses::OutputItem::Message(message) => message
            .content
            .iter()
            .map(translate_response_message_content)
            .collect(),
        responses::OutputItem::FunctionCall(call) => {
            vec![ContentBlock::ToolUse(tool_use_block(
                &call.call_id,
                &call.name,
                parse_json_or_string(&call.arguments),
            ))]
        }
        responses::OutputItem::CustomToolCall(call) => vec![ContentBlock::ToolUse(tool_use_block(
            &call.call_id,
            &call.name,
            parse_json_or_string(&call.input),
        ))],
        responses::OutputItem::Reasoning(reasoning) => translate_reasoning_item(reasoning, scope),
        other => {
            scope.dropped(
                format!("Responses output item `{}`", other.as_ref()),
                "Responses output item has no Anthropic Messages response representation",
            );
            Vec::new()
        }
    }
}

fn translate_response_message_content(content: &responses::OutputMessageContent) -> ContentBlock {
    match content {
        responses::OutputMessageContent::OutputText(text) => {
            ContentBlock::Text(text_block(&text.text))
        }
        responses::OutputMessageContent::Refusal(refusal) => {
            ContentBlock::Text(text_block(&refusal.refusal))
        }
    }
}

fn translate_reasoning_item(
    reasoning: &responses::ReasoningItem,
    scope: &TranslationScope,
) -> Vec<ContentBlock> {
    let mut blocks = reasoning
        .summary
        .iter()
        .map(|part| match part {
            responses::SummaryPart::SummaryText(text) => {
                scope.adapted(
                    "Responses reasoning summary_text",
                    "Anthropic Messages has no response-side summary block; mapping to thinking",
                );
                ContentBlock::Thinking(thinking_block(&text.text))
            }
        })
        .chain(reasoning.content.iter().flatten().map(|part| match part {
            responses::ReasoningItemContent::ReasoningText(text) => {
                ContentBlock::Thinking(thinking_block(&text.text))
            }
        }))
        .collect::<Vec<_>>();

    if let Some(data) = reasoning.encrypted_content.as_non_null() {
        blocks.push(ContentBlock::RedactedThinking(redacted_thinking_block(
            data,
        )));
    }

    blocks
}

impl From<&responses::ResponseUsage> for Usage {
    fn from(usage: &responses::ResponseUsage) -> Self {
        Self {
            cache_creation: None.into(),
            cache_creation_input_tokens: Some(usage.input_tokens_details.cache_write_tokens).into(),
            cache_read_input_tokens: Some(usage.input_tokens_details.cached_tokens).into(),
            inference_geo: None.into(),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            output_tokens_details: (usage.output_tokens_details.reasoning_tokens > 0)
                .then_some(OutputTokensDetails {
                    thinking_tokens: usage.output_tokens_details.reasoning_tokens,
                })
                .into(),
            server_tool_use: None.into(),
            service_tier: None.into(),
        }
    }
}

impl From<&responses::ResponseUsage> for MessageDeltaUsage {
    fn from(usage: &responses::ResponseUsage) -> Self {
        Self {
            cache_creation_input_tokens: Some(usage.input_tokens_details.cache_write_tokens).into(),
            cache_read_input_tokens: Some(usage.input_tokens_details.cached_tokens).into(),
            input_tokens: Some(usage.input_tokens).into(),
            output_tokens: usage.output_tokens,
            output_tokens_details: (usage.output_tokens_details.reasoning_tokens > 0)
                .then_some(OutputTokensDetails {
                    thinking_tokens: usage.output_tokens_details.reasoning_tokens,
                })
                .into(),
            server_tool_use: None.into(),
        }
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
