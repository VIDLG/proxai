//! Request translation for `anthropic_messages -> openai_chat_completions`.

mod messages;
mod reasoning;
mod tools;
mod types;

use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::translation::{TranslationError, TranslationResult};

use self::messages::chat_messages;
use self::reasoning::request_reasoning_effort;
use self::tools::chat_tool_config;
use self::types::chat_stop_configuration;

impl TryFrom<anthropic::MessageCreateParamsBase> for chat::CreateChatCompletionRequest {
    type Error = TranslationError;

    fn try_from(request: anthropic::MessageCreateParamsBase) -> TranslationResult<Self> {
        if request.messages.is_empty() {
            return Err(TranslationError::InvalidPayload(
                "Anthropic Messages request must contain at least one user or assistant message to translate to Chat Completions"
                    .to_string(),
            ));
        }

        let mut messages = Vec::new();

        if let Some(system) = request.system {
            messages.push(system.into());
        }

        for message in request.messages {
            messages.extend(chat_messages(message)?);
        }
        if messages.is_empty() {
            return Err(TranslationError::InvalidPayload(
                "Anthropic Messages request must contain at least one message to translate to Chat Completions"
                    .to_string(),
            ));
        }

        let tool_config = chat_tool_config(request.tools, request.tool_choice)?;

        let metadata = request.metadata.and_then(|metadata| {
            metadata
                .user_id
                .map(|user_id| std::collections::HashMap::from([("user_id".to_string(), user_id)]))
        });
        let safety_identifier = metadata
            .as_ref()
            .and_then(|metadata| metadata.get("user_id").cloned());

        let reasoning_effort =
            request_reasoning_effort(request.output_config.as_ref(), request.thinking.as_ref());
        let response_format = request
            .output_config
            .as_ref()
            .and_then(|config| config.format.clone().map(Into::into));

        Ok(Self {
            messages,
            model: request.model,
            modalities: None,
            verbosity: None,
            reasoning_effort,
            max_completion_tokens: Some(request.max_tokens),
            frequency_penalty: None,
            presence_penalty: None,
            web_search_options: None,
            top_logprobs: None,
            response_format,
            audio: None,
            store: None,
            stream: request.stream,
            stop: chat_stop_configuration(request.stop_sequences),
            logit_bias: None,
            logprobs: None,
            max_tokens: None,
            n: None,
            prediction: None,
            stream_options: None,
            service_tier: request.service_tier.map(Into::into),
            temperature: request
                .temperature
                .and_then(|number| number.as_f64().map(|value| value as f32)),
            top_p: request
                .top_p
                .and_then(|number| number.as_f64().map(|value| value as f32)),
            tools: tool_config.tools,
            tool_choice: tool_config.tool_choice,
            parallel_tool_calls: tool_config.parallel_tool_calls,
            safety_identifier,
            prompt_cache_key: None,
            metadata,
        })
    }
}
