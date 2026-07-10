use crate::protocol::openai::chat_completions::{
    ChatChoice, ChatCompletionMessageToolCalls, CreateChatCompletionResponse, FinishReason,
};
use crate::protocol::openai::responses::{
    OutputItem, OutputMessage, OutputMessageContent, Response,
};
use crate::translation::openai_chat_completions::outbound::{
    CHAT_COMPLETION_OBJECT, assistant_response_message,
};
use crate::translation::openai_responses::stop::{ResponsesStopKind, infer_response_stop_kind};
use crate::translation::{TranslationError, TranslationResult};

use super::types::chat_id;

impl TryFrom<&Response> for CreateChatCompletionResponse {
    type Error = TranslationError;

    fn try_from(response: &Response) -> TranslationResult<Self> {
        let mut content_parts: Vec<String> = Vec::new();
        let mut refusal_parts: Vec<String> = Vec::new();
        let mut tool_calls = Vec::new();

        for output in &response.output {
            match output {
                OutputItem::Message(message) => {
                    collect_message_content(message, &mut content_parts, &mut refusal_parts);
                }
                OutputItem::FunctionCall(call) => {
                    tool_calls.push(ChatCompletionMessageToolCalls::from(call));
                }
                OutputItem::CustomToolCall(call) => {
                    tool_calls.push(ChatCompletionMessageToolCalls::from(call));
                }
                skipped => {
                    // Responses output items without a Chat Completions
                    // representation (reasoning, hosted tool calls, MCP, search,
                    // compaction, etc.) are skipped with a trace log to keep
                    // silent drops observable.
                    tracing::trace!(
                        discriminant = ?std::mem::discriminant(skipped),
                        "skipping Responses output item with no Chat-representable field"
                    );
                }
            }
        }

        let content = content_parts.join("");
        let refusal = refusal_parts.join("");
        if !content.is_empty() && !refusal.is_empty() {
            return Err(TranslationError::InvalidPayload(
                "OpenAI Responses output contains both text and refusal content; Chat Completions response cannot represent mixed refusal semantics"
                    .to_string(),
            ));
        }
        if content.is_empty() && refusal.is_empty() && tool_calls.is_empty() {
            return Err(TranslationError::InvalidPayload(
                "OpenAI Responses output has no Chat-representable text or tool calls".to_string(),
            ));
        }

        Ok(Self {
            // Keep the upstream id embedded while presenting an OpenAI-shaped id.
            id: chat_id(&response.id),
            choices: vec![ChatChoice {
                index: 0,
                message: assistant_response_message(
                    (!content.is_empty()).then_some(content),
                    (!refusal.is_empty()).then_some(refusal),
                    (!tool_calls.is_empty()).then_some(tool_calls),
                    None,
                ),
                finish_reason: chat_finish_reason(response),
                logprobs: None,
            }],
            // Responses responses carry a `created_at` Unix timestamp.
            created: response.created_at as u32,
            model: response.model.clone(),
            // Responses has no Chat-style service tier field on the response body.
            service_tier: None,
            object: CHAT_COMPLETION_OBJECT.to_string(),
            usage: response.usage.as_ref().map(Into::into),
        })
    }
}
fn chat_finish_reason(response: &Response) -> Option<FinishReason> {
    infer_response_stop_kind(response).map(|kind| match kind {
        ResponsesStopKind::EndTurn | ResponsesStopKind::Refusal => FinishReason::Stop,
        ResponsesStopKind::MaxTokens => FinishReason::Length,
        ResponsesStopKind::ToolUse => FinishReason::ToolCalls,
    })
}

fn collect_message_content(
    message: &OutputMessage,
    content_parts: &mut Vec<String>,
    refusal_parts: &mut Vec<String>,
) {
    for content in &message.content {
        match content {
            OutputMessageContent::OutputText(text) => content_parts.push(text.text.clone()),
            OutputMessageContent::Refusal(refusal) => refusal_parts.push(refusal.refusal.clone()),
        }
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
