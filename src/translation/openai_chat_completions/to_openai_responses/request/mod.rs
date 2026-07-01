//! Request translation for `openai_chat_completions -> openai_responses`.

mod messages;
mod tools;
mod types;

use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai_responses as responses;
use crate::translation::TranslationResult;

use self::messages::{ResponsesInput, responses_input_from_messages};

impl TryFrom<&chat::CreateChatCompletionRequest> for responses::ResponseCreateParams {
    type Error = crate::translation::TranslationError;

    fn try_from(request: &chat::CreateChatCompletionRequest) -> TranslationResult<Self> {
        let ResponsesInput {
            instructions,
            items,
        } = responses_input_from_messages(request.messages.as_slice())?;

        let text_format = request
            .response_format
            .as_ref()
            .map(responses::TextResponseFormatConfiguration::try_from)
            .transpose()?;
        let text_verbosity = request.verbosity.map(Into::into);
        let text = (text_format.is_some() || text_verbosity.is_some()).then(|| {
            responses::ResponseTextParam {
                format: text_format.unwrap_or_default(),
                verbosity: text_verbosity,
            }
        });

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

        Ok(Self {
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
            background: None,
            conversation: None,
            include: None,
            max_tool_calls: None,
            previous_response_id: None,
            prompt: None,
            prompt_cache_retention: None,
            truncation: None,

            // Chat has direct equivalents for the remaining fields.
            input: Some(responses::InputParam::Items(items)),
            instructions,
            max_output_tokens: request.max_completion_tokens.or(request.max_tokens),
            metadata: request.metadata.clone(),
            model: Some(request.model.clone()),
            parallel_tool_calls: request.parallel_tool_calls,
            prompt_cache_key: request.prompt_cache_key.clone(),
            reasoning: request.reasoning_effort.map(Into::into),
            safety_identifier: request.safety_identifier.clone(),
            service_tier: request.service_tier.map(Into::into),
            store: request.store,
            stream: request.stream,
            stream_options: request.stream_options.map(Into::into),
            temperature: request.temperature,
            text,
            tool_choice: request
                .tool_choice
                .as_ref()
                .map(responses::ToolChoiceParam::try_from)
                .transpose()?,
            tools,
            top_logprobs: request.top_logprobs,
            top_p: request.top_p,
        })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
