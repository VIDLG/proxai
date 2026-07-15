//! Request translation for `openai_chat_completions -> openai_responses`.

mod messages;
mod tools;
mod types;

use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai_responses as responses;
use crate::translation::openai_chat_completions::compatibility::ChatRequestExtensions;
use crate::translation::{TranslationError, TranslationResult, TranslationScope};

use self::messages::{ResponsesInput, responses_input_from_messages};

pub(super) fn translate_request(
    request: &chat::CreateChatCompletionRequest,
    extensions: &ChatRequestExtensions,
    scope: &TranslationScope,
) -> TranslationResult<responses::CreateResponseRequest> {
    if request.function_call.is_some() || request.functions.is_some() {
        return Err(TranslationError::InvalidPayload(
                "deprecated Chat Completions function_call/functions must be migrated to tool_choice/tools before translating to OpenAI Responses"
                    .to_string(),
            ));
    }
    let ResponsesInput {
        instructions,
        items,
    } = responses_input_from_messages(request.messages.as_slice(), extensions)?;

    let text_format = request
        .response_format
        .as_ref()
        .map(responses::TextResponseFormatConfiguration::try_from)
        .transpose()?;
    let text_verbosity = request
        .verbosity
        .as_non_null()
        .copied()
        .map(responses::Verbosity::from);
    let text = (text_format.is_some() || text_verbosity.is_some()).then_some(
        responses::ResponseTextParam {
            format: text_format,
            verbosity: text_verbosity.into(),
        },
    );

    let tools = request
        .tools
        .as_ref()
        .map(|tools| {
            tools
                .iter()
                .map(responses::Tool::try_from)
                .collect::<TranslationResult<Vec<_>>>()
        })
        .transpose()?
        .filter(|tools| !tools.is_empty());

    if request.seed.is_non_null() {
        scope.dropped(
            "Chat Completions seed",
            "OpenAI Responses has no seed equivalent",
        );
    }

    Ok(responses::CreateResponseRequest {
        // Responses-only fields with no Chat Completions equivalent;
        // the source protocol carries no signal to populate them.
        // - `background`: Responses background-response flag.
        // - `conversation`: Responses conversation handle (stateful,
        //   keyed by `previous_response_id`).
        // - `include`: Responses expand directives (e.g. step_details).
        // - `max_tool_calls`: Responses tool-call budget.
        // - `previous_response_id`: Responses stateful chaining; Chat is
        //   stateless and replays the full message history.
        // - `prompt`: Responses prompt field; Chat's equivalent is the
        //   `messages` array, translated into `input` below.
        // - `prompt_cache_retention`: Responses cache retention policy;
        //   Chat only exposes `prompt_cache_key`.
        // - `truncation`: Responses auto-truncation strategy.
        background: None.into(),
        conversation: None.into(),
        context_management: None.into(),
        include: None.into(),
        max_tool_calls: None.into(),
        previous_response_id: None.into(),
        prompt: None.into(),
        prompt_cache_retention: request
            .prompt_cache_retention
            .as_non_null()
            .copied()
            .map(responses::PromptCacheRetention::from)
            .into(),
        truncation: None.into(),

        // Chat has direct equivalents for the remaining fields.
        input: Some(responses::InputParam::Items(items)),
        instructions: instructions.into(),
        max_output_tokens: request
            .max_completion_tokens
            .as_non_null()
            .copied()
            .or_else(|| request.max_tokens.as_non_null().copied())
            .into(),
        metadata: request.metadata.as_non_null().cloned().into(),
        model: Some(request.model.clone()),
        parallel_tool_calls: request.parallel_tool_calls.into(),
        prompt_cache_key: request.prompt_cache_key.clone(),
        reasoning: request
            .reasoning_effort
            .as_non_null()
            .copied()
            .map(responses::Reasoning::from)
            .into(),
        safety_identifier: request.safety_identifier.clone(),
        service_tier: request
            .service_tier
            .as_non_null()
            .copied()
            .map(responses::ServiceTier::from)
            .into(),
        store: request.store.as_non_null().copied().into(),
        stream: request.stream.as_non_null().copied().into(),
        stream_options: request
            .stream_options
            .as_non_null()
            .map(|options| types::response_stream_options(options, scope))
            .into(),
        temperature: request.temperature.as_non_null().copied().into(),
        text,
        tool_choice: request
            .tool_choice
            .as_ref()
            .map(responses::ToolChoiceParam::try_from)
            .transpose()?,
        tools,
        top_logprobs: request.top_logprobs.as_non_null().copied(),
        top_p: request.top_p.as_non_null().copied().into(),
        user: request.user.clone(),
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
