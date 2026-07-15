//! Request translation for `anthropic_messages -> openai_responses`.

mod messages;
mod reasoning;
mod tools;
mod types;

use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use crate::translation::{TranslationResult, TranslationScope};

use self::messages::translate_message_param;
use self::reasoning::request_reasoning;
use self::tools::responses_tool_config;

pub(super) fn translate_request(
    request: anthropic::MessageCreateParamsBase,
    scope: &TranslationScope,
) -> TranslationResult<responses::CreateResponseRequest> {
    let mut input_items: Vec<responses::InputItem> = Vec::new();

    let instructions = request.system.map(|system| String::from(&system));
    let reasoning = request_reasoning(
        request.output_config.as_ref(),
        request.thinking.as_ref(),
        scope,
    );
    let text = request
        .output_config
        .as_ref()
        .and_then(|config| config.format.as_non_null())
        .cloned()
        .map(responses::ResponseTextParam::try_from)
        .transpose()?;
    let metadata = request.metadata.and_then(|metadata| {
        metadata
            .user_id
            .into_non_null()
            .map(|user_id| std::collections::HashMap::from([("user_id".to_string(), user_id)]))
    });
    let safety_identifier = metadata
        .as_ref()
        .and_then(|metadata| metadata.get("user_id").cloned());
    let service_tier = request.service_tier.map(responses::ServiceTier::from);
    let tool_config = responses_tool_config(request.tools, request.tool_choice)?;

    for message in request.messages {
        input_items.extend(translate_message_param(message)?);
    }

    Ok(responses::CreateResponseRequest {
        background: None.into(),
        conversation: None.into(),
        context_management: None.into(),
        include: None.into(),
        input: Some(responses::InputParam::Items(input_items)),
        instructions: instructions.into(),
        max_output_tokens: Some(request.max_tokens).into(),
        max_tool_calls: None.into(),
        metadata: metadata.into(),
        model: Some(request.model),
        parallel_tool_calls: tool_config.parallel_tool_calls.into(),
        previous_response_id: None.into(),
        prompt: None.into(),
        prompt_cache_key: None,
        prompt_cache_retention: None.into(),
        reasoning: reasoning.into(),
        safety_identifier,
        service_tier: service_tier.into(),
        store: None.into(),
        stream: request.stream.into(),
        stream_options: None.into(),
        temperature: request
            .temperature
            .and_then(|number| number.as_f64().map(|value| value as f32))
            .into(),
        text,
        tool_choice: tool_config.tool_choice,
        tools: tool_config.tools,
        top_logprobs: None,
        top_p: request
            .top_p
            .and_then(|number| number.as_f64().map(|value| value as f32))
            .into(),
        truncation: None.into(),
        user: None,
    })
}

#[cfg(test)]
#[path = "../request_tests.rs"]
mod tests;
