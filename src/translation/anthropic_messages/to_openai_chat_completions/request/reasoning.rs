use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use tracing::warn;

pub(super) fn request_reasoning_effort(
    output_config: Option<&anthropic::OutputConfig>,
    thinking: Option<&anthropic::ThinkingConfigParam>,
) -> Option<chat::ReasoningEffort> {
    let output_reasoning_effort = output_config.and_then(|config| config.effort.map(Into::into));
    let has_output_reasoning_effort = output_reasoning_effort.is_some();
    let reasoning_effort = output_reasoning_effort.or_else(|| {
        thinking_effort(thinking).map(|(budget_tokens, effort)| {
            if let Some(budget_tokens) = budget_tokens {
                warn!(
                    event = "anthropic_legacy_thinking_budget_mapped",
                    budget_tokens,
                    reasoning_effort = ?effort,
                    target_protocol = "openai_chat_completions",
                    "mapped Anthropic legacy thinking.type=enabled budget_tokens lossily to reasoning effort"
                );
            }
            effort
        })
    });
    if has_output_reasoning_effort {
        warn_if_legacy_thinking_ignored(thinking);
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

fn warn_if_legacy_thinking_ignored(thinking: Option<&anthropic::ThinkingConfigParam>) {
    if let Some(anthropic::ThinkingConfigParam::Enabled(value)) = thinking {
        warn!(
            event = "anthropic_legacy_thinking_budget_ignored",
            budget_tokens = value.budget_tokens,
            target_protocol = "openai_chat_completions",
            "ignored Anthropic legacy thinking.type=enabled budget_tokens because output_config.effort is present"
        );
    }
}

impl From<anthropic::OutputEffort> for chat::ReasoningEffort {
    fn from(effort: anthropic::OutputEffort) -> Self {
        match effort {
            anthropic::OutputEffort::Low => Self::Low,
            anthropic::OutputEffort::Medium => Self::Medium,
            anthropic::OutputEffort::High => Self::High,
            anthropic::OutputEffort::Xhigh | anthropic::OutputEffort::Max => Self::Xhigh,
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
