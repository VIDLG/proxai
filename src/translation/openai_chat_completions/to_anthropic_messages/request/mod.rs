//! Request-level conversion for `openai_chat_completions -> anthropic_messages`.

mod documents;
mod messages;
mod reasoning;
mod tools;
mod types;

use self::messages::AnthropicMessages;
use self::reasoning::{output_config, thinking_config};
use self::tools::translate_tool_choice;
use self::types::{chat_max_tokens, stop_sequences};
use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::translation::anthropic_messages::outbound::json_number_from_f32;
use crate::translation::openai_chat_completions::compatibility::ChatRequestExtensions;
use crate::translation::{TranslationResult, TranslationScope};

pub(super) fn translate_request(
    request: &chat::CreateChatCompletionRequest,
    extensions: &ChatRequestExtensions,
    scope: &TranslationScope,
) -> TranslationResult<anthropic::MessageCreateParamsBase> {
    if request.function_call.is_some() || request.functions.is_some() {
        return Err(crate::translation::TranslationError::InvalidPayload(
                "deprecated Chat Completions function_call/functions must be migrated to tool_choice/tools before translating to Anthropic Messages"
                    .to_string(),
            ));
    }
    let anthropic_messages =
        AnthropicMessages::from_chat(request.messages.as_slice(), extensions, scope)?;

    let tools = request
        .tools
        .as_ref()
        .map(|tools| {
            tools
                .iter()
                .map(anthropic::ToolUnion::try_from)
                .collect::<TranslationResult<Vec<_>>>()
                .map(|tools| (!tools.is_empty()).then_some(tools))
        })
        .transpose()?
        .flatten();

    Ok(anthropic::MessageCreateParamsBase {
        max_tokens: chat_max_tokens(request),
        messages: anthropic_messages.messages,
        model: request.model.clone(),
        cache_control: None.into(),
        container: None.into(),
        inference_geo: None.into(),
        metadata: None,
        output_config: output_config(request.reasoning_effort.as_non_null().copied()),
        service_tier: None,
        stop_sequences: stop_sequences(request.stop.as_non_null()),
        stream: request.stream.as_non_null().copied(),
        system: anthropic_messages.system,
        temperature: request
            .temperature
            .as_non_null()
            .copied()
            .and_then(json_number_from_f32),
        thinking: request
            .reasoning_effort
            .as_non_null()
            .copied()
            .and_then(thinking_config),
        tool_choice: request
            .tool_choice
            .as_ref()
            .map(translate_tool_choice)
            .transpose()?
            .flatten(),
        tools,
        top_k: None,
        top_p: request
            .top_p
            .as_non_null()
            .copied()
            .and_then(json_number_from_f32),
    })
}

#[cfg(test)]
#[path = "../request_tests.rs"]
mod tests;
