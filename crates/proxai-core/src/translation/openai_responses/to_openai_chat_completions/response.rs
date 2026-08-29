use crate::protocol::openai::chat_completions::{
    ChatChoice, ChatCompletionMessageToolCalls, CreateChatCompletionResponse,
    CreateChatCompletionResponseObject,
};
use crate::protocol::openai::responses::{
    OutputItem, OutputMessageContent, ReasoningItemContent, Response, SummaryPart,
};
use crate::translation::openai_chat_completions::outbound::assistant_response_message;
use crate::translation::openai_responses::stop::infer_response_stop_kind;
use crate::translation::{TranslationError, TranslationResult, TranslationScope};

pub(super) struct ChatResponseProjection {
    pub(super) response: CreateChatCompletionResponse,
    pub(super) reasoning: Option<String>,
}

#[derive(Default)]
struct ChatResponseOutput {
    content: String,
    reasoning: String,
    refusal: String,
    tool_calls: Vec<ChatCompletionMessageToolCalls>,
}

impl ChatResponseOutput {
    fn project(mut self, output: &OutputItem, scope: &TranslationScope) -> ChatResponseOutput {
        match output {
            OutputItem::Message(message) => {
                for content in &message.content {
                    match content {
                        OutputMessageContent::OutputText(text) => {
                            self.content.push_str(&text.text);
                        }
                        OutputMessageContent::Refusal(refusal) => {
                            self.refusal.push_str(&refusal.refusal);
                        }
                    }
                }
            }
            OutputItem::FunctionCall(call) => {
                self.tool_calls
                    .push(ChatCompletionMessageToolCalls::from(call));
            }
            OutputItem::CustomToolCall(call) => {
                self.tool_calls
                    .push(ChatCompletionMessageToolCalls::from(call));
            }
            OutputItem::Reasoning(reasoning) => {
                for part in &reasoning.summary {
                    let SummaryPart::SummaryText(text) = part;
                    self.reasoning.push_str(&text.text);
                }
                if let Some(content) = reasoning.content.as_ref() {
                    for part in content {
                        let ReasoningItemContent::ReasoningText(text) = part;
                        self.reasoning.push_str(&text.text);
                    }
                }
                if reasoning.encrypted_content.is_non_null()
                    && reasoning.summary.is_empty()
                    && reasoning.content.as_ref().is_none_or(Vec::is_empty)
                {
                    scope.dropped(
                        "Responses encrypted reasoning item",
                        "Chat reasoning_content cannot represent encrypted reasoning without visible text",
                    );
                }
            }
            skipped => scope.dropped(
                format!("Responses output item `{}`", skipped.as_ref()),
                "Responses output item has no Chat Completions response representation",
            ),
        }
        self
    }
}

pub(super) fn translate_response_payload(
    response: &Response,
    scope: &TranslationScope,
) -> TranslationResult<ChatResponseProjection> {
    let output = response
        .output
        .iter()
        .fold(ChatResponseOutput::default(), |output, item| {
            output.project(item, scope)
        });

    let finish_reason = infer_response_stop_kind(response, scope)
        .map(Into::into)
        .ok_or_else(|| {
        TranslationError::InvalidPayload(
            "OpenAI Responses response has no terminal state required for Chat Completions finish_reason"
                .to_string(),
            )
        })?;
    let reasoning = (!output.reasoning.is_empty()).then_some(output.reasoning);
    let translated = CreateChatCompletionResponse {
        // Keep the upstream id embedded while presenting an OpenAI-shaped id.
        id: chat_id(&response.id),
        choices: vec![ChatChoice {
            index: 0,
            message: assistant_response_message(
                (!output.content.is_empty()).then_some(output.content),
                (!output.refusal.is_empty()).then_some(output.refusal),
                (!output.tool_calls.is_empty()).then_some(output.tool_calls),
                None,
            ),
            finish_reason,
            logprobs: None.into(),
        }],
        // Responses responses carry a `created_at` Unix timestamp.
        created: response.created_at as u32,
        model: response.model.clone(),
        service_tier: response
            .service_tier
            .as_non_null()
            .copied()
            .map(Into::into)
            .into(),
        system_fingerprint: None,
        object: CreateChatCompletionResponseObject::ChatCompletion,
        usage: response.usage.as_ref().map(Into::into),
        moderation: None.into(),
    };

    Ok(ChatResponseProjection {
        response: translated,
        reasoning,
    })
}

/// Normalize a Responses id into a Chat-shaped id.
///
/// Pair-local naming convention, not a protocol conversion: it just makes the
/// id start with `chatcmpl_` so downstream consumers recognize it. Lives here
/// rather than in pair-root `types.rs` because only response translation uses it.
fn chat_id(response_id: &str) -> String {
    if response_id.starts_with("chatcmpl_") {
        response_id.to_string()
    } else {
        format!("chatcmpl_{response_id}")
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
