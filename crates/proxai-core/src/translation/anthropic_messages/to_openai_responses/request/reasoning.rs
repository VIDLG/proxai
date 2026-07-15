use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use crate::translation::TranslationScope;

pub(super) fn request_reasoning(
    output_config: Option<&anthropic::OutputConfig>,
    thinking: Option<&anthropic::ThinkingConfigParam>,
    scope: &TranslationScope,
) -> Option<responses::Reasoning> {
    let summary = thinking.and_then(reasoning_summary);
    let output_effort = output_config
        .and_then(|config| config.effort.as_non_null())
        .copied()
        .map(Into::into);
    let has_output_effort = output_effort.is_some();
    let effort =
        output_effort.or_else(|| thinking.and_then(|thinking| thinking_effort(thinking, scope)));
    if has_output_effort {
        observe_legacy_thinking_ignored(thinking, scope);
    }

    if effort.is_some() || summary.is_some() {
        Some(responses::Reasoning {
            effort: effort.into(),
            generate_summary: None.into(),
            summary: summary.into(),
        })
    } else {
        None
    }
}

fn thinking_effort(
    thinking: &anthropic::ThinkingConfigParam,
    scope: &TranslationScope,
) -> Option<responses::ReasoningEffort> {
    match thinking {
        anthropic::ThinkingConfigParam::Enabled(value) => {
            let effort = responses::ReasoningEffort::from(value);
            scope.adapted(
                format!("Anthropic legacy thinking budget `{}`", value.budget_tokens),
                format!("mapped lossily to Responses reasoning effort `{effort:?}`"),
            );
            Some(effort)
        }
        anthropic::ThinkingConfigParam::Adaptive(_) => None,
        anthropic::ThinkingConfigParam::Disabled(_) => Some(responses::ReasoningEffort::None),
    }
}

fn observe_legacy_thinking_ignored(
    thinking: Option<&anthropic::ThinkingConfigParam>,
    scope: &TranslationScope,
) {
    if let Some(anthropic::ThinkingConfigParam::Enabled(value)) = thinking {
        scope.dropped(
            format!("Anthropic legacy thinking budget `{}`", value.budget_tokens),
            "output_config.effort takes precedence",
        );
    }
}

fn reasoning_summary(
    thinking: &anthropic::ThinkingConfigParam,
) -> Option<responses::ReasoningSummary> {
    let display = match thinking {
        anthropic::ThinkingConfigParam::Enabled(thinking) => thinking.display.as_non_null(),
        anthropic::ThinkingConfigParam::Adaptive(thinking) => thinking.display.as_non_null(),
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
