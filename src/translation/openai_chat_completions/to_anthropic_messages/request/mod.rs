//! Request-level conversion for `openai_chat_completions -> anthropic_messages`.

mod documents;
mod messages;
mod reasoning;
mod tools;
mod types;

use self::messages::AnthropicMessages;
use self::reasoning::{output_config, thinking_config};
use self::tools::translate_tool_choice;
use self::types::{chat_max_tokens, json_number_from_f32, stop_sequences};
use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::translation::TranslationResult;

impl TryFrom<&chat::CreateChatCompletionRequest> for anthropic::MessageCreateParamsBase {
    type Error = crate::translation::TranslationError;

    fn try_from(request: &chat::CreateChatCompletionRequest) -> TranslationResult<Self> {
        let anthropic_messages = AnthropicMessages::try_from(request.messages.as_slice())?;

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

        Ok(Self {
            max_tokens: chat_max_tokens(request),
            messages: anthropic_messages.messages,
            model: request.model.clone(),
            cache_control: None,
            container: None,
            inference_geo: None,
            metadata: None,
            output_config: output_config(request.reasoning_effort),
            service_tier: None,
            stop_sequences: stop_sequences(request.stop.as_ref()),
            stream: request.stream,
            system: anthropic_messages.system,
            temperature: request.temperature.and_then(json_number_from_f32),
            thinking: request.reasoning_effort.and_then(thinking_config),
            tool_choice: request
                .tool_choice
                .as_ref()
                .map(translate_tool_choice)
                .transpose()?
                .flatten(),
            tools,
            top_k: None,
            top_p: request.top_p.and_then(json_number_from_f32),
        })
    }
}
