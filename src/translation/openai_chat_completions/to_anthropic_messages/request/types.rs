use serde_json::Number;

use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;

// Anthropic Messages requires `max_tokens`, while Chat Completions allows both
// token-limit fields to be omitted. This is a proxai compatibility fallback for
// OpenAI-compatible clients that omit output limits; it is not an upstream
// OpenAI or Anthropic protocol default.
const COMPATIBILITY_MAX_TOKENS_FALLBACK: u32 = 4096;

pub(super) fn chat_max_tokens(request: &chat::CreateChatCompletionRequest) -> u32 {
    // Prefer the current Chat field. `max_tokens` is deprecated but still common
    // in OpenAI-compatible clients, so keep it as a fallback.
    request
        .max_completion_tokens
        .or(request.max_tokens)
        .unwrap_or(COMPATIBILITY_MAX_TOKENS_FALLBACK)
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

impl From<chat::ReasoningEffort> for Option<anthropic::ThinkingConfigParam> {
    fn from(effort: chat::ReasoningEffort) -> Self {
        match effort {
            chat::ReasoningEffort::None | chat::ReasoningEffort::Minimal => Some(
                anthropic::ThinkingConfigParam::Disabled(anthropic::ThinkingConfigDisabled),
            ),
            chat::ReasoningEffort::Low => Some(anthropic::ThinkingConfigParam::Enabled(
                anthropic::ThinkingConfigEnabled {
                    budget_tokens: 1024,
                    display: None,
                },
            )),
            chat::ReasoningEffort::Medium => Some(anthropic::ThinkingConfigParam::Enabled(
                anthropic::ThinkingConfigEnabled {
                    budget_tokens: 4096,
                    display: None,
                },
            )),
            chat::ReasoningEffort::High => Some(anthropic::ThinkingConfigParam::Enabled(
                anthropic::ThinkingConfigEnabled {
                    budget_tokens: 8192,
                    display: None,
                },
            )),
            chat::ReasoningEffort::Xhigh => Some(anthropic::ThinkingConfigParam::Enabled(
                anthropic::ThinkingConfigEnabled {
                    budget_tokens: 16384,
                    display: None,
                },
            )),
        }
    }
}

pub(super) fn json_number_from_f32(value: f32) -> Option<Number> {
    serde_json::Number::from_f64(value as f64)
}

pub(super) fn stop_sequences(value: Option<&chat::StopConfiguration>) -> Option<Vec<String>> {
    match value? {
        chat::StopConfiguration::String(value) if !value.is_empty() => Some(vec![value.clone()]),
        chat::StopConfiguration::StringArray(values) => {
            let values = values
                .iter()
                .filter(|value| !value.is_empty())
                .cloned()
                .collect::<Vec<_>>();
            (!values.is_empty()).then_some(values)
        }
        _ => None,
    }
}
