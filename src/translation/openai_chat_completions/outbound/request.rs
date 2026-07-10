//! Request-side OpenAI Chat Completions constructors.
//!
//! Used by `* -> openai_chat_completions` request translators to build
//! protocol-native request messages without repeating optional-field boilerplate.

use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::chat_completions::request::wire::ChatCompletionRequestMessageContentPartText;

pub(crate) fn text_part(text: impl Into<String>) -> ChatCompletionRequestMessageContentPartText {
    ChatCompletionRequestMessageContentPartText { text: text.into() }
}

pub(crate) fn system_message(
    content: chat::ChatCompletionRequestSystemMessageContent,
) -> chat::ChatCompletionRequestMessage {
    chat::ChatCompletionRequestMessage::System(chat::ChatCompletionRequestSystemMessage {
        content,
        name: None,
    })
}

pub(crate) fn system_text_message(text: impl Into<String>) -> chat::ChatCompletionRequestMessage {
    system_message(chat::ChatCompletionRequestSystemMessageContent::Text(
        text.into(),
    ))
}

pub(crate) fn developer_text_message(
    text: impl Into<String>,
) -> chat::ChatCompletionRequestMessage {
    chat::ChatCompletionRequestMessage::Developer(chat::ChatCompletionRequestDeveloperMessage {
        content: chat::ChatCompletionRequestDeveloperMessageContent::Text(text.into()),
        name: None,
    })
}

pub(crate) fn user_text_message(text: impl Into<String>) -> chat::ChatCompletionRequestMessage {
    user_message(chat::ChatCompletionRequestUserMessageContent::Text(
        text.into(),
    ))
}

pub(crate) fn user_message(
    content: chat::ChatCompletionRequestUserMessageContent,
) -> chat::ChatCompletionRequestMessage {
    chat::ChatCompletionRequestMessage::User(chat::ChatCompletionRequestUserMessage {
        content,
        name: None,
    })
}

pub(crate) fn assistant_message(
    content: Option<chat::ChatCompletionRequestAssistantMessageContent>,
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<chat::ChatCompletionMessageToolCalls>>,
) -> chat::ChatCompletionRequestMessage {
    chat::ChatCompletionRequestMessage::Assistant(chat::ChatCompletionRequestAssistantMessage {
        content,
        refusal: None,
        name: None,
        audio: None,
        reasoning_content,
        tool_calls,
    })
}

pub(crate) fn assistant_text_message(
    text: impl Into<String>,
) -> chat::ChatCompletionRequestMessage {
    assistant_message(
        Some(chat::ChatCompletionRequestAssistantMessageContent::Text(
            text.into(),
        )),
        None,
        None,
    )
}

pub(crate) fn tool_message(
    content: chat::ChatCompletionRequestToolMessageContent,
    tool_call_id: impl Into<String>,
) -> chat::ChatCompletionRequestMessage {
    chat::ChatCompletionRequestMessage::Tool(chat::ChatCompletionRequestToolMessage {
        content,
        tool_call_id: tool_call_id.into(),
    })
}
