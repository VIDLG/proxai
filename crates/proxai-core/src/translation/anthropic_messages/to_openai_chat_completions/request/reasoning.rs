use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::translation::TranslationScope;

pub(super) fn request_reasoning_effort(
    output_config: Option<&anthropic::OutputConfig>,
    thinking: Option<&anthropic::ThinkingConfigParam>,
    scope: &TranslationScope,
) -> Option<chat::ReasoningEffort> {
    let output_reasoning_effort = output_config
        .and_then(|config| config.effort.as_non_null())
        .copied()
        .map(Into::into);
    let has_output_reasoning_effort = output_reasoning_effort.is_some();
    let reasoning_effort = output_reasoning_effort.or_else(|| {
        thinking_effort(thinking).map(|(budget_tokens, effort)| {
            if let Some(budget_tokens) = budget_tokens {
                scope.adapted(
                    format!("Anthropic legacy thinking budget `{budget_tokens}`"),
                    format!("mapped lossily to Chat reasoning effort `{effort:?}`"),
                );
            }
            effort
        })
    });
    if has_output_reasoning_effort {
        observe_legacy_thinking_ignored(thinking, scope);
    }

    reasoning_effort
}

fn thinking_effort(
    thinking: Option<&anthropic::ThinkingConfigParam>,
) -> Option<(Option<u32>, chat::ReasoningEffort)> {
    match thinking? {
        anthropic::ThinkingConfigParam::Enabled(value) => Some((
            Some(value.budget_tokens),
            chat::ReasoningEffort::from(value),
        )),
        anthropic::ThinkingConfigParam::Adaptive(_) => None,
        anthropic::ThinkingConfigParam::Disabled(_) => Some((None, chat::ReasoningEffort::None)),
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

impl From<anthropic::OutputEffort> for chat::ReasoningEffort {
    fn from(effort: anthropic::OutputEffort) -> Self {
        match effort {
            anthropic::OutputEffort::Low => Self::Low,
            anthropic::OutputEffort::Medium => Self::Medium,
            anthropic::OutputEffort::High => Self::High,
            anthropic::OutputEffort::Xhigh => Self::Xhigh,
            anthropic::OutputEffort::Max => Self::Max,
        }
    }
}

impl From<&anthropic::ThinkingConfigEnabled> for chat::ReasoningEffort {
    fn from(thinking: &anthropic::ThinkingConfigEnabled) -> Self {
        match thinking.budget_tokens {
            1024..=2047 => Self::Low,
            2048..=8191 => Self::Medium,
            8192..=32767 => Self::High,
            _ => Self::Xhigh,
        }
    }
}
