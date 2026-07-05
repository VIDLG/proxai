use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;

use crate::translation::{TranslationError, TranslationResult};

impl From<responses::ReasoningSummary> for anthropic::ThinkingDisplay {
    fn from(summary: responses::ReasoningSummary) -> Self {
        match summary {
            responses::ReasoningSummary::Auto => Self::Summarized,
            responses::ReasoningSummary::Concise | responses::ReasoningSummary::Detailed => {
                tracing::trace!(
                    summary = %summary,
                    reason = "Anthropic Messages thinking display cannot distinguish concise/detailed Responses summaries; using summarized"
                );
                Self::Summarized
            }
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
