use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;

pub(super) fn output_config(
    effort: Option<chat::ReasoningEffort>,
) -> Option<anthropic::OutputConfig> {
    effort.and_then(|effort| {
        Option::<anthropic::OutputEffort>::from(effort).map(|effort| anthropic::OutputConfig {
            effort: Some(effort),
            format: None,
        })
    })
}

pub(super) fn thinking_config(
    effort: chat::ReasoningEffort,
) -> Option<anthropic::ThinkingConfigParam> {
    match effort {
        chat::ReasoningEffort::None | chat::ReasoningEffort::Minimal => Some(
            anthropic::ThinkingConfigParam::Disabled(anthropic::ThinkingConfigDisabled),
        ),
        chat::ReasoningEffort::Low
        | chat::ReasoningEffort::Medium
        | chat::ReasoningEffort::High
        | chat::ReasoningEffort::Xhigh => None,
    }
}

impl From<chat::ReasoningEffort> for Option<anthropic::OutputEffort> {
    fn from(effort: chat::ReasoningEffort) -> Self {
        match effort {
            chat::ReasoningEffort::None => None,
            chat::ReasoningEffort::Minimal | chat::ReasoningEffort::Low => {
                Some(anthropic::OutputEffort::Low)
            }
            chat::ReasoningEffort::Medium => Some(anthropic::OutputEffort::Medium),
            chat::ReasoningEffort::High => Some(anthropic::OutputEffort::High),
            chat::ReasoningEffort::Xhigh => Some(anthropic::OutputEffort::Xhigh),
        }
    }
}
