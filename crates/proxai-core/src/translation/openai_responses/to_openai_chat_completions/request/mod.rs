//! Request translation for `openai_responses -> openai_chat_completions`.

mod content;
mod messages;
mod tools;
mod types;

use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::responses;
use crate::translation::openai_chat_completions::compatibility::ChatRequestExtensions;

use crate::translation::{TranslationError, TranslationResult, TranslationScope};

use self::messages::project_request_messages;
use self::tools::{chat_tool_choice, chat_tools};

pub(super) struct ChatRequestProjection {
    pub(super) request: chat::CreateChatCompletionRequest,
    pub(super) extensions: ChatRequestExtensions,
}

pub(super) fn translate_request(
    request: &responses::CreateResponseRequest,
    scope: &TranslationScope,
) -> TranslationResult<ChatRequestProjection> {
    let (messages, extensions) = project_request_messages(
        request.instructions.as_non_null().map(String::as_str),
        request.input.as_ref(),
        scope,
    )?;

    let tools = chat_tools(request.tools.as_deref(), scope);
    let tool_choice = chat_tool_choice(request.tool_choice.as_ref(), scope);
    if let Some(reasoning) = request.reasoning.as_non_null() {
        if reasoning.mode.is_some() {
            scope.dropped(
                "Responses reasoning.mode",
                "Chat Completions reasoning_effort has no execution-mode field",
            );
        }
        if reasoning.summary.is_non_null() {
            scope.dropped(
                "Responses reasoning.summary",
                "Chat Completions has no reasoning-summary request control",
            );
        }
        if reasoning.context.is_non_null() {
            scope.dropped(
                "Responses reasoning.context",
                "Chat Completions has no reasoning history-selection control",
            );
        }
        if reasoning.generate_summary.is_non_null() {
            scope.dropped(
                "Responses reasoning.generate_summary",
                "Chat Completions has no reasoning-summary request control",
            );
        }
    }

    // Responses `text.format` maps onto Chat `response_format`.
    let response_format = request
        .text
        .as_ref()
        .and_then(|text| text.format.as_ref())
        .map(chat::ResponseFormat::try_from)
        .transpose()?;

    let translated = chat::CreateChatCompletionRequest {
        messages,
        model: request.model.clone().ok_or_else(|| {
            TranslationError::InvalidPayload(
                "OpenAI Responses request without `model` cannot be translated to Chat Completions"
                    .to_string(),
            )
        })?,
        modalities: None.into(),
        verbosity: None.into(),
        reasoning_effort: request
            .reasoning
            .clone()
            .map(|reasoning| chat::ReasoningEffort::from(&reasoning)),
        max_completion_tokens: request.max_output_tokens,
        frequency_penalty: None.into(),
        presence_penalty: None.into(),
        web_search_options: None,
        top_logprobs: request.top_logprobs.into(),
        response_format,
        audio: None.into(),
        store: request.store,
        moderation: request.moderation.clone(),
        stream: request.stream,
        stop: None.into(),
        logit_bias: None.into(),
        logprobs: None.into(),
        max_tokens: None.into(),
        n: None.into(),
        prediction: None.into(),
        prompt_cache_retention: request
            .prompt_cache_retention
            .map(chat::PromptCacheRetention::from),
        prompt_cache_options: request.prompt_cache_options.clone(),
        seed: None.into(),
        stream_options: request
            .stream_options
            .clone()
            .map(|options| chat::ChatCompletionStreamOptions::from(&options)),
        service_tier: request.service_tier.map(chat::ServiceTier::from),
        temperature: request.temperature,
        top_p: request.top_p,
        tools,
        tool_choice,
        parallel_tool_calls: request.parallel_tool_calls.as_non_null().copied(),
        function_call: None,
        functions: None,
        safety_identifier: request.safety_identifier.clone(),
        prompt_cache_key: request.prompt_cache_key.clone(),
        user: request.user.clone(),
        metadata: request.metadata.clone(),
    };

    Ok(ChatRequestProjection {
        request: translated,
        extensions,
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
