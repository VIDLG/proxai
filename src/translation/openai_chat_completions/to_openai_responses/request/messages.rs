use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::chat_completions::request::wire::ChatCompletionRequestMessageContentPartText;
use crate::protocol::openai_responses as responses;
use crate::translation::{TranslationError, TranslationResult};

pub(super) struct ResponsesInput {
    pub instructions: Option<String>,
    pub items: Vec<responses::InputItem>,
}

pub(super) fn responses_input_from_messages(
    chat_messages: &[chat::ChatCompletionRequestMessage],
) -> TranslationResult<ResponsesInput> {
    let mut instruction_parts = Vec::new();
    let mut items = Vec::new();

    for (message_index, message) in chat_messages.iter().enumerate() {
        match message {
            chat::ChatCompletionRequestMessage::Developer(message) => {
                instruction_parts.extend(developer_text_parts(&message.content));
            }
            chat::ChatCompletionRequestMessage::System(message) => {
                instruction_parts.extend(system_text_parts(&message.content));
            }
            chat::ChatCompletionRequestMessage::User(message) => {
                items.push(easy_message(
                    responses::Role::User,
                    (&message.content).try_into()?,
                ));
            }
            chat::ChatCompletionRequestMessage::Assistant(message) => {
                items.extend(assistant_input_items(message, message_index)?);
            }
            chat::ChatCompletionRequestMessage::Tool(message) => {
                items.push(responses::InputItem::Item(
                    responses::Item::FunctionCallOutput(responses::FunctionCallOutputItemParam {
                        call_id: message.tool_call_id.clone(),
                        output: responses::FunctionCallOutput::try_from(&message.content)?,
                        id: None,
                        status: None,
                    }),
                ));
            }
            chat::ChatCompletionRequestMessage::Function(message) => {
                return Err(TranslationError::InvalidPayload(format!(
                    "Chat Completions legacy function message `{}` cannot be translated to OpenAI Responses because it does not carry a tool_call_id",
                    message.name
                )));
            }
        }
    }

    if items.is_empty() {
        return Err(TranslationError::InvalidPayload(
            "Chat Completions request without user, assistant, or tool messages cannot be translated to OpenAI Responses input"
                .to_string(),
        ));
    }

    let instructions = instruction_parts
        .into_iter()
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(ResponsesInput {
        instructions: (!instructions.is_empty()).then_some(instructions),
        items,
    })
}

fn developer_text_parts(
    content: &chat::ChatCompletionRequestDeveloperMessageContent,
) -> Vec<String> {
    match content {
        chat::ChatCompletionRequestDeveloperMessageContent::Text(text) => vec![text.clone()],
        chat::ChatCompletionRequestDeveloperMessageContent::Array(parts) => parts
            .iter()
            .map(|part| match part {
                chat::ChatCompletionRequestDeveloperMessageContentPart::Text(part) => {
                    part.text.clone()
                }
            })
            .collect(),
    }
}

fn system_text_parts(content: &chat::ChatCompletionRequestSystemMessageContent) -> Vec<String> {
    match content {
        chat::ChatCompletionRequestSystemMessageContent::Text(text) => vec![text.clone()],
        chat::ChatCompletionRequestSystemMessageContent::Array(parts) => parts
            .iter()
            .map(|part| match part {
                chat::ChatCompletionRequestSystemMessageContentPart::Text(part) => {
                    part.text.clone()
                }
            })
            .collect(),
    }
}

fn easy_message(
    role: responses::Role,
    content: responses::EasyInputContent,
) -> responses::InputItem {
    responses::InputItem::EasyMessage(responses::EasyInputMessage {
        r#type: responses::MessageType::Message,
        role,
        content,
        phase: None,
    })
}

impl TryFrom<&chat::ChatCompletionRequestUserMessageContent> for responses::EasyInputContent {
    type Error = TranslationError;

    fn try_from(
        content: &chat::ChatCompletionRequestUserMessageContent,
    ) -> TranslationResult<Self> {
        match content {
            chat::ChatCompletionRequestUserMessageContent::Text(text) => {
                if text.is_empty() {
                    return Err(TranslationError::InvalidPayload(
                        "Chat Completions user message text content cannot be empty when translating to OpenAI Responses input"
                            .to_string(),
                    ));
                }
                Ok(Self::Text(text.clone()))
            }
            chat::ChatCompletionRequestUserMessageContent::Array(parts) => {
                if parts.is_empty() {
                    return Err(TranslationError::InvalidPayload(
                        "Chat Completions user message content array cannot be empty when translating to OpenAI Responses input"
                            .to_string(),
                    ));
                }
                Ok(Self::ContentList(
                    parts
                        .iter()
                        .map(responses::InputContent::try_from)
                        .collect::<TranslationResult<Vec<_>>>()?,
                ))
            }
        }
    }
}

fn assistant_input_items(
    message: &chat::ChatCompletionRequestAssistantMessage,
    message_index: usize,
) -> TranslationResult<Vec<responses::InputItem>> {
    let mut items = Vec::new();
    let mut content = Vec::new();

    if let Some(message_content) = message.content.as_ref() {
        content.extend(assistant_output_content(message_content)?);
    }

    if let Some(refusal) = message.refusal.as_ref() {
        if refusal.is_empty() {
            return Err(TranslationError::InvalidPayload(
                "Chat Completions assistant refusal content cannot be empty when translating to OpenAI Responses input"
                    .to_string(),
            ));
        }
        content.push(responses::OutputMessageContent::Refusal(
            responses::RefusalContent {
                refusal: refusal.clone(),
            },
        ));
    }

    if !content.is_empty() {
        items.push(responses::InputItem::Item(responses::Item::Message(
            responses::MessageItem::Output(responses::OutputMessage {
                id: format!("msg_chat_assistant_{message_index}"),
                role: responses::AssistantRole::Assistant,
                status: responses::OutputStatus::Completed,
                content,
                phase: None,
            }),
        )));
    }

    if let Some(tool_calls) = message.tool_calls.as_ref() {
        for tool_call in tool_calls {
            items.push(responses::InputItem::Item(tool_call.into()));
        }
    }

    if items.is_empty() {
        return Err(TranslationError::InvalidPayload(
            "Chat Completions assistant message without content, refusal, or tool calls cannot be translated to OpenAI Responses input"
                .to_string(),
        ));
    }

    Ok(items)
}

fn assistant_output_content(
    content: &chat::ChatCompletionRequestAssistantMessageContent,
) -> TranslationResult<Vec<responses::OutputMessageContent>> {
    match content {
        chat::ChatCompletionRequestAssistantMessageContent::Text(text) => {
            if text.is_empty() {
                return Err(TranslationError::InvalidPayload(
                    "Chat Completions assistant message text content cannot be empty when translating to OpenAI Responses input"
                        .to_string(),
                ));
            }
            Ok(vec![responses::OutputMessageContent::OutputText(
                responses::OutputTextContent {
                    text: text.clone(),
                    annotations: Vec::new(),
                    logprobs: None,
                },
            )])
        }
        chat::ChatCompletionRequestAssistantMessageContent::Array(parts) => {
            if parts.is_empty() {
                return Err(TranslationError::InvalidPayload(
                    "Chat Completions assistant message content array cannot be empty when translating to OpenAI Responses input"
                        .to_string(),
                ));
            }
            parts
                .iter()
                .map(responses::OutputMessageContent::try_from)
                .collect::<TranslationResult<Vec<_>>>()
        }
    }
}

impl TryFrom<&chat::ChatCompletionRequestAssistantMessageContentPart>
    for responses::OutputMessageContent
{
    type Error = TranslationError;

    fn try_from(
        part: &chat::ChatCompletionRequestAssistantMessageContentPart,
    ) -> TranslationResult<Self> {
        match part {
            chat::ChatCompletionRequestAssistantMessageContentPart::Text(part) => {
                if part.text.is_empty() {
                    return Err(TranslationError::InvalidPayload(
                        "Chat Completions assistant message text content part cannot be empty when translating to OpenAI Responses input"
                            .to_string(),
                    ));
                }
                Ok(part.into())
            }
            chat::ChatCompletionRequestAssistantMessageContentPart::Refusal(part) => {
                if part.refusal.is_empty() {
                    return Err(TranslationError::InvalidPayload(
                        "Chat Completions assistant refusal content part cannot be empty when translating to OpenAI Responses input"
                            .to_string(),
                    ));
                }
                Ok(part.into())
            }
        }
    }
}

impl From<&ChatCompletionRequestMessageContentPartText> for responses::OutputMessageContent {
    fn from(part: &ChatCompletionRequestMessageContentPartText) -> Self {
        Self::OutputText(responses::OutputTextContent {
            text: part.text.clone(),
            annotations: Vec::new(),
            logprobs: None,
        })
    }
}

impl From<&chat::ChatCompletionRequestMessageContentPartRefusal>
    for responses::OutputMessageContent
{
    fn from(part: &chat::ChatCompletionRequestMessageContentPartRefusal) -> Self {
        Self::Refusal(responses::RefusalContent {
            refusal: part.refusal.clone(),
        })
    }
}

impl TryFrom<&chat::ChatCompletionRequestUserMessageContentPart> for responses::InputContent {
    type Error = TranslationError;

    fn try_from(
        part: &chat::ChatCompletionRequestUserMessageContentPart,
    ) -> TranslationResult<Self> {
        match part {
            chat::ChatCompletionRequestUserMessageContentPart::Text(part) => {
                if part.text.is_empty() {
                    return Err(TranslationError::InvalidPayload(
                        "Chat Completions user message text content part cannot be empty when translating to OpenAI Responses input"
                            .to_string(),
                    ));
                }
                Ok(responses::InputContent::from(part))
            }
            chat::ChatCompletionRequestUserMessageContentPart::ImageUrl(part) => {
                Ok(responses::InputContent::InputImage(responses::InputImageContent {
                    detail: part.image_url.detail.map(Into::into),
                    file_id: None,
                    image_url: Some(part.image_url.url.clone()),
                }))
            }
            chat::ChatCompletionRequestUserMessageContentPart::File(part) => {
                Ok(responses::InputContent::InputFile(responses::InputFileContent {
                    file_data: part.file.file_data.clone(),
                    file_id: part.file.file_id.clone(),
                    file_url: None,
                    filename: part.file.filename.clone(),
                    detail: None,
                }))
            }
            chat::ChatCompletionRequestUserMessageContentPart::InputAudio(_) => Err(
                TranslationError::InvalidPayload(
                    "Chat Completions input_audio user content cannot be translated to OpenAI Responses input content"
                        .to_string(),
                ),
            ),
        }
    }
}

impl From<&ChatCompletionRequestMessageContentPartText> for responses::InputContent {
    fn from(part: &ChatCompletionRequestMessageContentPartText) -> Self {
        Self::InputText(responses::InputTextContent {
            text: part.text.clone(),
        })
    }
}

impl From<chat::ImageDetail> for responses::ImageDetail {
    fn from(value: chat::ImageDetail) -> Self {
        match value {
            chat::ImageDetail::Auto => Self::Auto,
            chat::ImageDetail::Low => Self::Low,
            chat::ImageDetail::High => Self::High,
            chat::ImageDetail::Original => Self::Original,
        }
    }
}

impl From<&chat::ChatCompletionMessageToolCalls> for responses::Item {
    fn from(value: &chat::ChatCompletionMessageToolCalls) -> Self {
        match value {
            chat::ChatCompletionMessageToolCalls::Function(call) => {
                responses::Item::FunctionCall(responses::FunctionToolCall {
                    arguments: call.function.arguments.clone(),
                    call_id: call.id.clone(),
                    namespace: None,
                    name: call.function.name.clone(),
                    id: Some(call.id.clone()),
                    status: None,
                })
            }
            chat::ChatCompletionMessageToolCalls::Custom(call) => {
                responses::Item::CustomToolCall(responses::CustomToolCall {
                    call_id: call.id.clone(),
                    namespace: None,
                    input: call.custom_tool.input.clone(),
                    name: call.custom_tool.name.clone(),
                    id: Some(call.id.clone()),
                })
            }
        }
    }
}

impl TryFrom<&chat::ChatCompletionRequestToolMessageContent> for responses::FunctionCallOutput {
    type Error = TranslationError;

    fn try_from(
        content: &chat::ChatCompletionRequestToolMessageContent,
    ) -> TranslationResult<Self> {
        match content {
            chat::ChatCompletionRequestToolMessageContent::Text(text) => {
                if text.is_empty() {
                    return Err(TranslationError::InvalidPayload(
                        "Chat Completions tool message text content cannot be empty when translating to OpenAI Responses function_call_output"
                            .to_string(),
                    ));
                }
                Ok(Self::Text(text.clone()))
            }
            chat::ChatCompletionRequestToolMessageContent::Array(parts) => {
                if parts.is_empty() {
                    return Err(TranslationError::InvalidPayload(
                        "Chat Completions tool message content array cannot be empty when translating to OpenAI Responses function_call_output"
                            .to_string(),
                    ));
                }
                let parts = parts
                    .iter()
                    .map(responses::InputContent::try_from)
                    .collect::<TranslationResult<Vec<_>>>()?;
                Ok(Self::Content(parts))
            }
        }
    }
}

impl TryFrom<&chat::ChatCompletionRequestToolMessageContentPart> for responses::InputContent {
    type Error = TranslationError;

    fn try_from(
        part: &chat::ChatCompletionRequestToolMessageContentPart,
    ) -> TranslationResult<Self> {
        match part {
            chat::ChatCompletionRequestToolMessageContentPart::Text(part) => {
                if part.text.is_empty() {
                    return Err(TranslationError::InvalidPayload(
                        "Chat Completions tool message text content part cannot be empty when translating to OpenAI Responses function_call_output"
                            .to_string(),
                    ));
                }
                Ok(part.into())
            }
        }
    }
}
