//! Request translation for `openai_responses -> openai_chat_completions`.

mod messages;
mod tools;
mod types;

use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::responses;
use crate::translation::{TranslationError, TranslationResult};

use self::messages::chat_messages;
use self::tools::chat_tools;

impl TryFrom<&responses::ResponseCreateParams> for chat::CreateChatCompletionRequest {
    type Error = TranslationError;

    fn try_from(request: &responses::ResponseCreateParams) -> TranslationResult<Self> {
        let messages = chat_messages(request.instructions.as_deref(), request.input.as_ref())?;

        let tools = chat_tools(&request.tools)?;
        let tool_choice = request
            .tool_choice
            .as_ref()
            .map(chat::ChatCompletionToolChoiceOption::try_from)
            .transpose()?;

        // Responses `text.format` maps onto Chat `response_format`.
        let response_format = match request.text.as_ref() {
            Some(text) => Some(chat::ResponseFormat::try_from(&text.format)?),
            None => None,
        };

        Ok(Self {
            messages,
            model: request
                .model
                .clone()
                .ok_or_else(|| TranslationError::InvalidPayload(
                    "OpenAI Responses request without `model` cannot be translated to Chat Completions"
                        .to_string(),
                ))?,
            modalities: None,
            verbosity: None,
            reasoning_effort: request.reasoning.as_ref().map(Into::into),
            max_completion_tokens: request.max_output_tokens,
            frequency_penalty: None,
            presence_penalty: None,
            web_search_options: None,
            top_logprobs: request.top_logprobs,
            response_format,
            audio: None,
            store: request.store,
            stream: request.stream,
            stop: None,
            logit_bias: None,
            logprobs: None,
            max_tokens: None,
            n: None,
            prediction: None,
            stream_options: request.stream_options.as_ref().map(Into::into),
            service_tier: request.service_tier.map(Into::into),
            temperature: request.temperature,
            top_p: request.top_p,
            tools,
            tool_choice,
            parallel_tool_calls: request.parallel_tool_calls,
            safety_identifier: request.safety_identifier.clone(),
            prompt_cache_key: request.prompt_cache_key.clone(),
            metadata: request.metadata.clone(),
        })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
