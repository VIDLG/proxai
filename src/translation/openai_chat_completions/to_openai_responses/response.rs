use crate::protocol::openai::chat_completions::CreateChatCompletionResponse;
use crate::protocol::openai::responses::{
    AssistantRole, OutputItem, OutputMessage, OutputMessageContent, OutputStatus,
    OutputTextContent, RefusalContent, Response, ResponseUsage,
};

use crate::translation::{TranslationError, TranslationResult};

use super::super::response::single_assistant_choice;

use super::types::{incomplete_details_from_finish_reason, response_id};

impl TryFrom<&CreateChatCompletionResponse> for Response {
    type Error = TranslationError;

    fn try_from(chat: &CreateChatCompletionResponse) -> TranslationResult<Self> {
        let choice = single_assistant_choice(&chat.choices)?;

        let mut output = Vec::new();
        let message = &choice.message;
        if message
            .content
            .as_ref()
            .is_some_and(|value| !value.is_empty())
            || message
                .refusal
                .as_ref()
                .is_some_and(|value| !value.is_empty())
        {
            let mut content = Vec::new();
            if let Some(text) = message.content.as_ref().filter(|value| !value.is_empty()) {
                content.push(OutputMessageContent::OutputText(OutputTextContent {
                    text: text.clone(),
                    annotations: Vec::new(),
                    logprobs: None,
                }));
            }
            if let Some(refusal) = message.refusal.as_ref().filter(|value| !value.is_empty()) {
                content.push(OutputMessageContent::Refusal(RefusalContent {
                    refusal: refusal.clone(),
                }));
            }
            output.push(OutputItem::Message(OutputMessage {
                id: format!("msg_{}_{}", chat.id, choice.index),
                role: AssistantRole::Assistant,
                status: OutputStatus::Completed,
                content,
                phase: None,
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
            background: None,
            billing: None,
            conversation: None,
            created_at: chat.created as u64,
            completed_at: None,
            error: None,
            id: response_id(&chat.id),
            incomplete_details: incomplete_details_from_finish_reason(choice.finish_reason),
            instructions: None,
            max_output_tokens: None,
            metadata: None,
            model: chat.model.clone(),
            object: "response".to_string(),
            output,
            parallel_tool_calls: None,
            previous_response_id: None,
            prompt: None,
            prompt_cache_key: None,
            prompt_cache_retention: None,
            reasoning: None,
            safety_identifier: None,
            service_tier: None,
            status: choice.finish_reason.map(Into::into).unwrap_or_default(),
            temperature: None,
            text: None,
            tool_choice: None,
            tools: None,
            top_logprobs: None,
            top_p: None,
            truncation: None,
            usage: chat.usage.as_ref().map(ResponseUsage::from),
        })
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
