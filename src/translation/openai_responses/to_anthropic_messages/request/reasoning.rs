use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;

use crate::translation::anthropic_messages::outbound::output_config as anthropic_output_config;
use crate::translation::{TranslationError, TranslationResult};

pub(super) fn output_config(
    reasoning: &responses::Reasoning,
) -> TranslationResult<Option<anthropic::OutputConfig>> {
    let Some(effort) = reasoning.effort else {
        return Ok(None);
    };
    Ok(Some(anthropic_output_config(effort.try_into()?)))
}

pub(super) fn thinking_config(
    reasoning: &responses::Reasoning,
) -> Option<anthropic::ThinkingConfigParam> {
    Some(anthropic::ThinkingConfigParam::Adaptive(
        anthropic::ThinkingConfigAdaptive {
            display: Some(thinking_display(reasoning.summary?)),
        },
    ))
}

fn thinking_display(summary: responses::ReasoningSummary) -> anthropic::ThinkingDisplay {
    match summary {
        responses::ReasoningSummary::Auto => anthropic::ThinkingDisplay::Summarized,
        responses::ReasoningSummary::Concise | responses::ReasoningSummary::Detailed => {
            tracing::trace!(
                summary = %summary,
                reason = "Anthropic Messages thinking display cannot distinguish concise/detailed Responses summaries; using summarized"
            );
            anthropic::ThinkingDisplay::Summarized
        }
    }
}

impl TryFrom<responses::ReasoningEffort> for anthropic::OutputEffort {
    type Error = TranslationError;

    fn try_from(effort: responses::ReasoningEffort) -> TranslationResult<Self> {
        match effort {
            responses::ReasoningEffort::Low => Ok(Self::Low),
            responses::ReasoningEffort::Medium => Ok(Self::Medium),
            responses::ReasoningEffort::High => Ok(Self::High),
            responses::ReasoningEffort::Xhigh => Ok(Self::Xhigh),
            responses::ReasoningEffort::None | responses::ReasoningEffort::Minimal => {
                Err(TranslationError::InvalidPayload(format!(
                    "OpenAI Responses reasoning effort `{effort}` cannot be translated to Anthropic output_config.effort"
                )))
            }
        }
    }
}
