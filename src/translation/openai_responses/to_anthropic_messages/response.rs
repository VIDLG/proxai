//! `openai_responses -> anthropic_messages` non-streaming response translation.
//!
//! Parses an OpenAI Responses `Response` JSON body into Anthropic `Message`
//! shape, including output items (text / tool calls / reasoning) and usage.

use crate::protocol::anthropic::messages::{
    ContentBlock, Message as AnthropicMessage, MessageRole, MessageType, StopReason, Usage,
};
use crate::protocol::openai_responses as responses;
use crate::translation::anthropic_messages::outbound::{
    redacted_thinking_block, text_block, thinking_block, tool_use_block,
};
use crate::translation::text::parse_json_or_string;

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;

pub(super) fn translate_response_payload(response: &responses::Response) -> AnthropicMessage {
    let content = response
        .output
        .iter()
        .flat_map(translate_response_output_item)
        .collect::<Vec<_>>();

    AnthropicMessage {
        id: response.id.clone(),
        container: None,
        content,
        model: response.model.clone(),
        role: MessageRole::Assistant,
        type_: MessageType::Message,
        stop_details: None,
        stop_reason: anthropic_stop_reason(response.status),
        stop_sequence: None,
        usage: anthropic_usage(response.usage.as_ref()),
    }
}

fn translate_response_output_item(item: &responses::OutputItem) -> Vec<ContentBlock> {
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
        responses::OutputItem::Reasoning(reasoning) => translate_reasoning_item(reasoning),
        other => {
            tracing::trace!(
                output_item_type = output_item_type(other),
                reason = "Responses output item has no Anthropic Messages response representation"
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

fn translate_reasoning_item(reasoning: &responses::ReasoningItem) -> Vec<ContentBlock> {
    let mut blocks = reasoning
        .summary
        .iter()
        .map(|part| match part {
            responses::SummaryPart::SummaryText(text) => {
                ContentBlock::Thinking(thinking_block(&text.text))
            }
        })
        .chain(reasoning.content.iter().flatten().map(|part| match part {
            responses::ReasoningItemContent::ReasoningText(text) => {
                ContentBlock::Thinking(thinking_block(&text.text))
            }
        }))
        .collect::<Vec<_>>();

    if let Some(data) = &reasoning.encrypted_content {
        blocks.push(ContentBlock::RedactedThinking(redacted_thinking_block(
            data,
        )));
    }

    blocks
}

fn anthropic_stop_reason(status: responses::Status) -> Option<StopReason> {
    match status {
        responses::Status::Completed => Some(StopReason::EndTurn),
        responses::Status::Incomplete => Some(StopReason::MaxTokens),
        responses::Status::Failed => Some(StopReason::Refusal),
        // Cancelled / Queued are client-side lifecycle states with no
        // Anthropic equivalent; emit no stop reason.
        responses::Status::Cancelled
        | responses::Status::Queued
        | responses::Status::InProgress => None,
    }
}

fn anthropic_usage(usage: Option<&responses::ResponseUsage>) -> Usage {
    Usage {
        cache_creation: None,
        cache_creation_input_tokens: None,
        cache_read_input_tokens: usage.map(|usage| usage.input_tokens_details.cached_tokens),
        inference_geo: None,
        input_tokens: usage.map(|usage| usage.input_tokens).unwrap_or_default(),
        output_tokens: usage.map(|usage| usage.output_tokens).unwrap_or_default(),
        output_tokens_details: None,
        server_tool_use: None,
        service_tier: None,
    }
}

fn output_item_type(item: &responses::OutputItem) -> &'static str {
    match item {
        responses::OutputItem::Message(_) => "message",
        responses::OutputItem::FileSearchCall(_) => "file_search_call",
        responses::OutputItem::FunctionCall(_) => "function_call",
        responses::OutputItem::FunctionCallOutput(_) => "function_call_output",
        responses::OutputItem::WebSearchCall(_) => "web_search_call",
        responses::OutputItem::ComputerCall(_) => "computer_call",
        responses::OutputItem::ComputerCallOutput(_) => "computer_call_output",
        responses::OutputItem::Reasoning(_) => "reasoning",
        responses::OutputItem::Compaction(_) => "compaction",
        responses::OutputItem::ImageGenerationCall(_) => "image_generation_call",
        responses::OutputItem::CodeInterpreterCall(_) => "code_interpreter_call",
        responses::OutputItem::LocalShellCall(_) => "local_shell_call",
        responses::OutputItem::ShellCall(_) => "shell_call",
        responses::OutputItem::ShellCallOutput(_) => "shell_call_output",
        responses::OutputItem::ApplyPatchCall(_) => "apply_patch_call",
        responses::OutputItem::ApplyPatchCallOutput(_) => "apply_patch_call_output",
        responses::OutputItem::McpCall(_) => "mcp_call",
        responses::OutputItem::McpListTools(_) => "mcp_list_tools",
        responses::OutputItem::McpApprovalRequest(_) => "mcp_approval_request",
        responses::OutputItem::CustomToolCall(_) => "custom_tool_call",
        responses::OutputItem::CustomToolCallOutput(_) => "custom_tool_call_output",
        responses::OutputItem::ToolSearchCall(_) => "tool_search_call",
        responses::OutputItem::ToolSearchOutput(_) => "tool_search_output",
    }
}
