//! Request translation for `anthropic_messages -> openai_chat_completions`.

mod messages;
mod reasoning;
mod tools;
mod types;

use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::translation::openai_chat_completions::compatibility::ChatRequestExtensions;
use crate::translation::{TranslationError, TranslationResult, TranslationScope};

use self::messages::{assistant_reasoning_content, chat_messages};
use self::reasoning::request_reasoning_effort;
use self::tools::chat_tool_config;
use self::types::chat_stop_configuration;

pub(super) fn translate_request(
    request: anthropic::MessageCreateParamsBase,
    scope: &TranslationScope,
) -> TranslationResult<(chat::CreateChatCompletionRequest, ChatRequestExtensions)> {
    if request.messages.is_empty() {
        return Err(TranslationError::InvalidPayload(
                "Anthropic Messages request must contain at least one user or assistant message to translate to Chat Completions"
                    .to_string(),
            ));
    }

    let mut messages = Vec::new();
    let mut extensions = ChatRequestExtensions::default();

    if let Some(system) = request.system {
        messages.push(system.into());
    }

    for message in request.messages {
        let reasoning = assistant_reasoning_content(&message)?;
        let message_index = messages.len();
        messages.extend(chat_messages(message)?);
        if let Some(reasoning) = reasoning {
            extensions.insert(message_index, reasoning);
        }
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
            .into_non_null()
            .map(|user_id| std::collections::HashMap::from([("user_id".to_string(), user_id)]))
    });
    let safety_identifier = metadata
        .as_ref()
        .and_then(|metadata| metadata.get("user_id").cloned());

    let reasoning_effort = request_reasoning_effort(
        request.output_config.as_ref(),
        request.thinking.as_ref(),
        scope,
    );
    let response_format = request
        .output_config
        .as_ref()
        .and_then(|config| config.format.clone().into_non_null())
        .map(Into::into);

    Ok((
        chat::CreateChatCompletionRequest {
            messages,
            model: request.model,
            modalities: None.into(),
            verbosity: None.into(),
            reasoning_effort: reasoning_effort.into(),
            max_completion_tokens: Some(request.max_tokens).into(),
            frequency_penalty: None.into(),
            presence_penalty: None.into(),
            web_search_options: None,
            top_logprobs: None.into(),
            response_format,
            audio: None.into(),
            store: None.into(),
            stream: request.stream.into(),
            stop: chat_stop_configuration(request.stop_sequences).into(),
            logit_bias: None.into(),
            logprobs: None.into(),
            max_tokens: None.into(),
            n: None.into(),
            prediction: None.into(),
            prompt_cache_retention: None.into(),
            seed: None.into(),
            stream_options: None.into(),
            service_tier: request.service_tier.map(chat::ServiceTier::from).into(),
            temperature: request
                .temperature
                .and_then(|number| number.as_f64().map(|value| value as f32))
                .into(),
            top_p: request
                .top_p
                .and_then(|number| number.as_f64().map(|value| value as f32))
                .into(),
            tools: tool_config.tools,
            tool_choice: tool_config.tool_choice,
            parallel_tool_calls: tool_config.parallel_tool_calls,
            function_call: None,
            functions: None,
            safety_identifier,
            prompt_cache_key: None,
            user: None,
            metadata: metadata.into(),
        },
        extensions,
    ))
}

#[cfg(test)]
#[path = "../request_tests.rs"]
mod tests;
