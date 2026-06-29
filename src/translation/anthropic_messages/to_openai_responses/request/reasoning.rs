use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use tracing::warn;

pub(super) fn request_reasoning(
    output_config: Option<&anthropic::OutputConfig>,
    thinking: Option<&anthropic::ThinkingConfigParam>,
) -> Option<responses::Reasoning> {
    let summary = thinking.and_then(reasoning_summary);
    let output_effort = output_config
        .and_then(|config| config.effort)
        .map(Into::into);
    let has_output_effort = output_effort.is_some();
    let effort = output_effort.or_else(|| thinking.and_then(thinking_effort));
    if has_output_effort {
        warn_if_legacy_thinking_ignored(thinking, "openai_responses");
    }

    if effort.is_some() || summary.is_some() {
        Some(responses::Reasoning { effort, summary })
    } else {
        None
    }
}

fn thinking_effort(
    thinking: &anthropic::ThinkingConfigParam,
) -> Option<responses::ReasoningEffort> {
    match thinking {
        anthropic::ThinkingConfigParam::Enabled(value) => {
            let effort = responses::ReasoningEffort::from(value);
            warn!(
                event = "anthropic_legacy_thinking_budget_mapped",
                budget_tokens = value.budget_tokens,
                reasoning_effort = ?effort,
                target_protocol = "openai_responses",
                "mapped Anthropic legacy thinking.type=enabled budget_tokens lossily to reasoning effort"
            );
            Some(effort)
        }
        anthropic::ThinkingConfigParam::Adaptive(_) => None,
        anthropic::ThinkingConfigParam::Disabled(_) => Some(responses::ReasoningEffort::None),
    }
}

fn warn_if_legacy_thinking_ignored(
    thinking: Option<&anthropic::ThinkingConfigParam>,
    target_protocol: &'static str,
) {
    if let Some(anthropic::ThinkingConfigParam::Enabled(value)) = thinking {
        warn!(
            event = "anthropic_legacy_thinking_budget_ignored",
            budget_tokens = value.budget_tokens,
            target_protocol,
            "ignored Anthropic legacy thinking.type=enabled budget_tokens because output_config.effort is present"
        );
    }
}

fn reasoning_summary(
    thinking: &anthropic::ThinkingConfigParam,
) -> Option<responses::ReasoningSummary> {
    let display = match thinking {
        anthropic::ThinkingConfigParam::Enabled(thinking) => thinking.display,
        anthropic::ThinkingConfigParam::Adaptive(thinking) => thinking.display,
        anthropic::ThinkingConfigParam::Disabled(_) => None,
    }?;

    match display {
        anthropic::ThinkingDisplay::Summarized => Some(responses::ReasoningSummary::Auto),
        anthropic::ThinkingDisplay::Omitted => None,
    }
}

impl From<anthropic::OutputEffort> for responses::ReasoningEffort {
    fn from(effort: anthropic::OutputEffort) -> Self {
        match effort {
            anthropic::OutputEffort::Low => Self::Low,
            anthropic::OutputEffort::Medium => Self::Medium,
            anthropic::OutputEffort::High => Self::High,
            anthropic::OutputEffort::Xhigh | anthropic::OutputEffort::Max => Self::Xhigh,
        }
    }
}

impl From<&anthropic::ThinkingConfigEnabled> for responses::ReasoningEffort {
    fn from(thinking: &anthropic::ThinkingConfigEnabled) -> Self {
        match thinking.budget_tokens {
            1024..=2047 => Self::Low,
            2048..=8191 => Self::Medium,
            8192..=32767 => Self::High,
            _ => Self::Xhigh,
        }
    }
}
