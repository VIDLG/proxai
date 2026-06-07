//! Request translation for `anthropic_messages -> openai_chat_completions`.

mod messages;
mod tools;
mod types;

use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::translation::{TranslationError, TranslationResult};

use self::messages::chat_messages;
use self::tools::chat_tool_config;
use self::types::chat_stop_configuration;

impl TryFrom<anthropic::MessageCreateParamsBase> for chat::CreateChatCompletionRequest {
    type Error = TranslationError;

    fn try_from(request: anthropic::MessageCreateParamsBase) -> TranslationResult<Self> {
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

        Ok(Self {
            messages,
            model: request.model,
            reasoning_effort: request
                .output_config
                .and_then(|config| config.effort.map(Into::into))
                .or_else(|| request.thinking.and_then(Into::into)),
            max_completion_tokens: Some(request.max_tokens),
            stream: request.stream,
            stop: chat_stop_configuration(request.stop_sequences),
            temperature: request
                .temperature
                .and_then(|number| number.as_f64().map(|value| value as f32)),
            top_p: request
                .top_p
                .and_then(|number| number.as_f64().map(|value| value as f32)),
            tools: tool_config.tools,
            tool_choice: tool_config.tool_choice,
            parallel_tool_calls: tool_config.parallel_tool_calls,
            ..Self::default()
        })
    }
}
