//! Response-side OpenAI Chat Completions constructors.
//!
//! Used by `* -> openai_chat_completions` response translators to build
//! protocol-native response messages without repeating optional-field boilerplate.

use crate::protocol::openai::chat_completions as chat;

pub(crate) fn assistant_response_message(
    content: Option<String>,
    refusal: Option<String>,
    tool_calls: Option<Vec<chat::ChatCompletionMessageToolCalls>>,
    annotations: Option<Vec<chat::ChatCompletionResponseMessageAnnotation>>,
) -> chat::ChatCompletionResponseMessage {
    chat::ChatCompletionResponseMessage {
        content: content.into(),
        refusal: refusal.into(),
        tool_calls,
        annotations,
        role: chat::AssistantRole::Assistant,
        function_call: None,
        audio: None.into(),
    }
}
