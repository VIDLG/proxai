use serde_json::Number;

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
