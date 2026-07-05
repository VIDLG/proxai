use crate::protocol::openai::chat_completions as chat;
use crate::translation::anthropic_messages::outbound::COMPATIBILITY_MAX_TOKENS_FALLBACK;

pub(super) fn chat_max_tokens(request: &chat::CreateChatCompletionRequest) -> u32 {
    // Prefer the current Chat field. `max_tokens` is deprecated but still common
    // in OpenAI-compatible clients, so keep it as a fallback.
    request
        .max_completion_tokens
        .or(request.max_tokens)
        .unwrap_or(COMPATIBILITY_MAX_TOKENS_FALLBACK)
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
