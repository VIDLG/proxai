//! `openai_responses -> anthropic_messages` request translation.

mod messages;
mod reasoning;
mod tools;
mod types;

use self::messages::translate_messages;
use self::reasoning::{output_config, thinking_config};
use self::tools::{translate_tool_choice, translate_tools};
use self::types::{DEFAULT_MAX_TOKENS, json_number_from_f32};
use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use crate::translation::{TranslationError, TranslationResult};

impl TryFrom<&responses::ResponseCreateParams> for anthropic::MessageCreateParamsBase {
    type Error = TranslationError;

    fn try_from(request: &responses::ResponseCreateParams) -> TranslationResult<Self> {
        let model = request.model.clone().ok_or_else(|| {
            TranslationError::InvalidPayload(
                "openai_responses -> anthropic_messages request requires `model`".to_string(),
            )
        })?;
        let (system, messages) = translate_messages(request)?;

        Ok(Self {
            max_tokens: request.max_output_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            messages,
            model,
            cache_control: None,
            container: None,
            inference_geo: None,
            metadata: request.metadata.as_ref().and_then(|metadata| {
                metadata.get("user_id").map(|user_id| anthropic::Metadata {
                    user_id: Some(user_id.clone()),
                })
            }),
            output_config: request
                .reasoning
                .as_ref()
                .map(output_config)
                .transpose()?
                .flatten(),
            service_tier: None,
            stop_sequences: None,
            stream: request.stream,
            system,
            temperature: request.temperature.and_then(json_number_from_f32),
            thinking: request.reasoning.as_ref().and_then(thinking_config),
            tool_choice: translate_tool_choice(
                request.tool_choice.as_ref(),
                request.parallel_tool_calls,
            ),
            tools: translate_tools(request.tools.as_ref()),
            top_k: None,
            top_p: request.top_p.and_then(json_number_from_f32),
        })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
