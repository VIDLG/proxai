//! `openai_responses -> anthropic_messages` request translation.

mod messages;
mod reasoning;
mod tools;

use self::messages::translate_messages;
use self::tools::translate_tool_choice;
use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use crate::translation::anthropic_messages::outbound::{
    COMPATIBILITY_MAX_TOKENS_FALLBACK, json_number_from_f32, output_config,
};
use crate::translation::{TranslationError, TranslationResult, TranslationScope};

pub(super) fn translate_request(
    request: &responses::CreateResponseRequest,
    scope: &TranslationScope,
) -> TranslationResult<anthropic::MessageCreateParamsBase> {
    let model = request.model.clone().ok_or_else(|| {
        TranslationError::InvalidPayload(
            "openai_responses -> anthropic_messages request requires `model`".to_string(),
        )
    })?;
    let (system, messages) = translate_messages(
        request.instructions.as_non_null().map(String::as_str),
        request.input.as_ref(),
        scope,
    )?;
    let reasoning = request.reasoning.as_non_null();
    if reasoning
        .and_then(|reasoning| reasoning.mode.as_ref())
        .is_some()
    {
        scope.dropped(
            "Responses reasoning.mode",
            "Anthropic Messages has no equivalent reasoning execution mode",
        );
    }
    if reasoning
        .and_then(|reasoning| reasoning.context.as_non_null())
        .is_some()
    {
        scope.dropped(
            "Responses reasoning.context",
            "Anthropic Messages has no equivalent reasoning history-selection control",
        );
    }
    let reasoning_summary = reasoning.and_then(|reasoning| {
        if let Some(summary) = reasoning.summary.as_non_null().copied() {
            if reasoning.generate_summary.is_non_null() {
                scope.dropped(
                    "Responses reasoning.generate_summary",
                    "reasoning.summary takes precedence over the deprecated field",
                );
            }
            return Some(summary);
        }
        let summary = reasoning.generate_summary.as_non_null().copied()?;
        scope.adapted(
            "Responses reasoning.generate_summary",
            "used as deprecated alias for reasoning.summary",
        );
        Some(summary)
    });
    if request.prompt_cache_options.is_some() {
        scope.dropped(
            "Responses prompt_cache_options",
            "Anthropic prompt caching uses a different cache-control model",
        );
    }
    if request.moderation.is_non_null() {
        scope.dropped(
            "Responses moderation",
            "Anthropic Messages has no request-level moderation configuration",
        );
    }

    Ok(anthropic::MessageCreateParamsBase {
        max_tokens: request
            .max_output_tokens
            .as_non_null()
            .copied()
            .unwrap_or(COMPATIBILITY_MAX_TOKENS_FALLBACK),
        messages,
        model,
        cache_control: None.into(),
        container: None.into(),
        inference_geo: None.into(),
        metadata: request.metadata.as_non_null().and_then(|metadata| {
            metadata.get("user_id").map(|user_id| anthropic::Metadata {
                user_id: Some(user_id.clone()).into(),
            })
        }),
        output_config: reasoning
            .and_then(|reasoning| reasoning.effort.as_non_null())
            .copied()
            .map(|effort| effort.try_into().map(output_config))
            .transpose()?,
        service_tier: None,
        stop_sequences: None,
        stream: request.stream.as_non_null().copied(),
        system,
        temperature: request
            .temperature
            .as_non_null()
            .copied()
            .and_then(json_number_from_f32),
        thinking: reasoning_summary.map(|summary| {
            anthropic::ThinkingConfigParam::Adaptive(anthropic::ThinkingConfigAdaptive {
                display: Some(reasoning::thinking_display(summary, scope)).into(),
            })
        }),
        tool_choice: request
            .tool_choice
            .as_ref()
            .map(|choice| {
                translate_tool_choice(
                    choice,
                    (request.parallel_tool_calls.as_non_null().copied() == Some(false))
                        .then_some(true),
                )
            })
            .transpose()?,
        tools: request
            .tools
            .as_ref()
            .map(|tools| {
                tools
                    .iter()
                    .map(anthropic::ToolUnion::try_from)
                    .collect::<TranslationResult<Vec<_>>>()
            })
            .transpose()?
            .filter(|tools| !tools.is_empty()),
        top_k: None,
        top_p: request
            .top_p
            .as_non_null()
            .copied()
            .and_then(json_number_from_f32),
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
