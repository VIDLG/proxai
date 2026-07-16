use crate::protocol::openai::chat_completions::CreateChatCompletionResponse;
use crate::protocol::openai::responses::{
    AssistantRole, OutputItem, OutputMessage, OutputMessageContent, OutputStatus, ReasoningItem,
    ReasoningItemContent, ReasoningTextContent, RefusalContent, Response, ResponseObject,
    ResponseUsage, ToolChoiceOptions, ToolChoiceParam,
};
use crate::translation::anthropic_messages::continuation::ContinuationEnvelope;
use crate::translation::openai_responses::outbound::{output_text, response_id};
use crate::translation::{TranslationError, TranslationResult, TranslationScope};

use super::super::response::single_assistant_choice;
use super::types::{
    incomplete_details_from_finish_reason, responses_status_from_chat_finish_reason,
};

pub(super) fn translate_response(
    chat: &CreateChatCompletionResponse,
    reasoning_content: Option<&str>,
    scope: &TranslationScope,
) -> TranslationResult<Response> {
    let choice = single_assistant_choice(&chat.choices)?;

    let mut output = Vec::new();
    if let Some(reasoning_content) = reasoning_content {
        let (visible_reasoning, continuation) =
            ContinuationEnvelope::split_chat_reasoning_content(reasoning_content)?;
        if continuation.is_some() {
            scope.dropped("Anthropic continuation envelope in Chat reasoning_content",
                "OpenAI Responses reasoning cannot carry provider-specific Anthropic continuation data",
            );
        }
        if !visible_reasoning.is_empty() {
            output.push(OutputItem::Reasoning(ReasoningItem {
                id: format!("rs_{}_{}", chat.id, choice.index),
                encrypted_content: None.into(),
                summary: Vec::new(),
                content: Some(vec![ReasoningItemContent::ReasoningText(
                    ReasoningTextContent {
                        text: visible_reasoning,
                    },
                )]),
                status: Some(OutputStatus::Completed),
            }));
        }
    }
    let message = &choice.message;
    if message.function_call.is_some() {
        return Err(TranslationError::InvalidPayload(
                "deprecated Chat response function_call cannot be translated to OpenAI Responses because it has no tool_call_id"
                    .to_string(),
            ));
    }
    if message
        .content
        .as_non_null()
        .is_some_and(|value| !value.is_empty())
        || message
            .refusal
            .as_non_null()
            .is_some_and(|value| !value.is_empty())
    {
        let mut content = Vec::new();
        if let Some(text) = message
            .content
            .as_non_null()
            .filter(|value| !value.is_empty())
        {
            content.push(OutputMessageContent::OutputText(output_text(
                text,
                Vec::new(),
            )));
        }
        if let Some(refusal) = message
            .refusal
            .as_non_null()
            .filter(|value| !value.is_empty())
        {
            content.push(OutputMessageContent::Refusal(RefusalContent {
                refusal: refusal.clone(),
            }));
        }
        output.push(OutputItem::Message(OutputMessage {
            id: format!("msg_{}_{}", chat.id, choice.index),
            role: AssistantRole::Assistant,
            status: OutputStatus::Completed,
            content,
            phase: None.into(),
        }));
    }
    if let Some(tool_calls) = message.tool_calls.as_ref() {
        for tool_call in tool_calls {
            output.push(OutputItem::from(tool_call));
        }
    }

    if output.is_empty() {
        return Err(TranslationError::InvalidPayload(
                "Chat Completions response without content, refusal, or tool calls cannot be translated to OpenAI Responses output"
                    .to_string(),
            ));
    }

    Ok(Response {
        background: None.into(),
        conversation: None.into(),
        created_at: chat.created as f64,
        completed_at: None.into(),
        error: None.into(),
        id: response_id(&chat.id),
        incomplete_details: incomplete_details_from_finish_reason(Some(choice.finish_reason))
            .into(),
        instructions: None.into(),
        moderation: None.into(),
        max_output_tokens: None.into(),
        max_tool_calls: None.into(),
        metadata: None.into(),
        model: chat.model.clone(),
        object: ResponseObject::Response,
        output,
        output_text: None.into(),
        parallel_tool_calls: false,
        previous_response_id: None.into(),
        prompt: None.into(),
        prompt_cache_key: None,
        prompt_cache_retention: None.into(),
        prompt_cache_options: None,
        reasoning: None.into(),
        safety_identifier: None,
        service_tier: None.into(),
        status: Some(responses_status_from_chat_finish_reason(
            choice.finish_reason,
        )),
        temperature: None.into(),
        text: None,
        tool_choice: ToolChoiceParam::Mode(ToolChoiceOptions::Auto),
        tools: Vec::new(),
        top_logprobs: None.into(),
        top_p: None.into(),
        truncation: None.into(),
        user: None,
        usage: chat.usage.as_ref().map(ResponseUsage::from),
    })
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
