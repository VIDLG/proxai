use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;

use crate::translation::{TranslationError, TranslationResult, TranslationScope};

pub(super) fn thinking_display(
    summary: responses::ReasoningSummary,
    scope: &TranslationScope,
) -> anthropic::ThinkingDisplay {
    if matches!(
        summary,
        responses::ReasoningSummary::Concise | responses::ReasoningSummary::Detailed
    ) {
        scope.adapted(
            format!("Responses reasoning summary `{summary}`"),
            "Anthropic thinking display cannot distinguish concise/detailed; using summarized",
        );
    }
    anthropic::ThinkingDisplay::Summarized
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
