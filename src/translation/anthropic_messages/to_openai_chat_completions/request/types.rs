use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;

pub(super) fn chat_stop_configuration(
    stop_sequences: Option<Vec<String>>,
) -> Option<chat::StopConfiguration> {
    let mut stop_sequences = stop_sequences?
        .into_iter()
        .filter(|sequence| !sequence.is_empty())
        .collect::<Vec<_>>();
    match stop_sequences.len() {
        0 => None,
        1 => stop_sequences.pop().map(chat::StopConfiguration::String),
        _ => Some(chat::StopConfiguration::StringArray(stop_sequences)),
    }
}

pub(super) fn non_empty<T>(items: Vec<T>) -> Option<Vec<T>> {
    (!items.is_empty()).then_some(items)
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

impl From<anthropic::ThinkingConfigEnabled> for chat::ReasoningEffort {
    fn from(thinking: anthropic::ThinkingConfigEnabled) -> Self {
        match thinking.budget_tokens {
            0..=2047 => Self::Low,
            2048..=8191 => Self::Medium,
            8192..=32767 => Self::High,
            _ => Self::Xhigh,
        }
    }
}

impl From<anthropic::ThinkingConfigAdaptive> for chat::ReasoningEffort {
    fn from(_thinking: anthropic::ThinkingConfigAdaptive) -> Self {
        Self::Medium
    }
}

impl From<anthropic::ThinkingConfigParam> for Option<chat::ReasoningEffort> {
    fn from(thinking: anthropic::ThinkingConfigParam) -> Self {
        match thinking {
            anthropic::ThinkingConfigParam::Enabled(thinking) => Some(thinking.into()),
            anthropic::ThinkingConfigParam::Adaptive(thinking) => Some(thinking.into()),
            anthropic::ThinkingConfigParam::Disabled(_) => Some(chat::ReasoningEffort::None),
        }
    }
}

impl AsRef<str> for anthropic::ImageMediaType {
    fn as_ref(&self) -> &str {
        match self {
            anthropic::ImageMediaType::Jpeg => "image/jpeg",
            anthropic::ImageMediaType::Png => "image/png",
            anthropic::ImageMediaType::Gif => "image/gif",
            anthropic::ImageMediaType::Webp => "image/webp",
        }
    }
}
