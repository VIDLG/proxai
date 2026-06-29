//! Request translation for `anthropic_messages -> openai_responses`.

mod messages;
mod reasoning;
mod tools;
mod types;

use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use crate::translation::TranslationResult;

use self::messages::translate_message_param;
use self::reasoning::request_reasoning;
use self::tools::responses_tool_config;

impl TryFrom<anthropic::MessageCreateParamsBase> for responses::ResponseCreateParams {
    type Error = crate::translation::TranslationError;

    fn try_from(request: anthropic::MessageCreateParamsBase) -> TranslationResult<Self> {
        let mut input_items: Vec<responses::InputItem> = Vec::new();

        let instructions = request.system.as_ref().map(Into::into);
        let reasoning =
            request_reasoning(request.output_config.as_ref(), request.thinking.as_ref());
        let text = request
            .output_config
            .and_then(|config| config.format)
            .map(TryInto::try_into)
            .transpose()?;
        let metadata = request.metadata.and_then(|metadata| {
            metadata
                .user_id
                .map(|user_id| std::collections::HashMap::from([("user_id".to_string(), user_id)]))
        });
        let safety_identifier = metadata
            .as_ref()
            .and_then(|metadata| metadata.get("user_id").cloned());
        let service_tier = request.service_tier.map(Into::into);
        let tool_config = responses_tool_config(request.tools, request.tool_choice)?;

        for message in request.messages {
            input_items.extend(translate_message_param(message)?);
        }

        Ok(Self {
            background: None,
            conversation: None,
            include: None,
            input: Some(responses::InputParam::Items(input_items)),
            instructions,
            max_output_tokens: Some(request.max_tokens),
            max_tool_calls: None,
            metadata,
            model: Some(request.model),
            parallel_tool_calls: tool_config.parallel_tool_calls,
            previous_response_id: None,
            prompt: None,
            prompt_cache_key: None,
            prompt_cache_retention: None,
            reasoning,
            safety_identifier,
            service_tier,
            store: None,
            stream: request.stream,
            stream_options: None,
            temperature: request
                .temperature
                .and_then(|number| number.as_f64().map(|value| value as f32)),
            text,
            tool_choice: tool_config.tool_choice,
            tools: tool_config.tools,
            top_logprobs: None,
            top_p: request
                .top_p
                .and_then(|number| number.as_f64().map(|value| value as f32)),
            truncation: None,
        })
    }
}
