use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::responses;
use crate::translation::openai_chat_completions::outbound::{
    assistant_message, developer_text_message, system_text_message, text_part, tool_message,
    user_message, user_text_message,
};
use crate::translation::{TranslationError, TranslationResult};

pub(super) fn chat_messages(
    instructions: Option<&str>,
    input: Option<&responses::InputParam>,
) -> TranslationResult<Vec<chat::ChatCompletionRequestMessage>> {
    let mut messages: Vec<chat::ChatCompletionRequestMessage> = Vec::new();
    if let Some(instructions) = instructions.map(str::trim)
        && !instructions.is_empty()
    {
        messages.push(developer_text_message(instructions));
    }

    match input {
        None => {}
        Some(responses::InputParam::Text(text)) => {
            messages.push(user_text_message(text));
        }
        Some(responses::InputParam::Items(items)) => {
            for item in items {
                chat_messages_for_input_item(item, &mut messages)?;
            }
        }
    }

    if !messages.iter().any(is_non_instruction_message) {
        // Chat Completions needs an actual conversational turn. A Responses
        // request can rely only on top-level `instructions`; keep those as
        // developer instructions and add an empty user turn for the model to answer.
        messages.push(user_text_message(""));
    }

    Ok(messages)
}

fn chat_messages_for_input_item(
    item: &responses::InputItem,
    messages: &mut Vec<chat::ChatCompletionRequestMessage>,
) -> TranslationResult<()> {
    match item {
        responses::InputItem::EasyMessage(message) => {
            messages_from_easy_message(message, messages)?;
        }
        responses::InputItem::Item(item) => {
            messages_from_item(item, messages)?;
        }
        responses::InputItem::ItemReference(reference) => {
            // Responses item references are stateful pointers that the proxy
            // cannot resolve; surface them as user text so the upstream is the
            // one deciding how to handle the dangling reference.
            messages.push(user_text_message(format!(
                "[OpenAI Responses item_reference `{}` omitted during Chat Completions translation]",
                reference.id
            )));
        }
    }
    Ok(())
}

fn messages_from_easy_message(
    message: &responses::EasyInputMessage,
    messages: &mut Vec<chat::ChatCompletionRequestMessage>,
) -> TranslationResult<()> {
    let role = message.role;
    match role {
        responses::Role::System => {
            for text in text_parts_from_easy_content(&message.content) {
                if !text.trim().is_empty() {
                    messages.push(system_text_message(text));
                }
            }
        }
        responses::Role::Developer => {
            for text in text_parts_from_easy_content(&message.content) {
                if !text.trim().is_empty() {
                    messages.push(developer_text_message(text));
                }
            }
        }
        responses::Role::User => {
            messages.push(user_message((&message.content).try_into()?));
        }
        // Easy input messages only carry user/system/developer/assistant roles.
        responses::Role::Assistant => {
            // Easy assistant content is treated as output text; Chat assistant
            // messages require non-empty content, so drop empty turns.
            if let Some(content) = assistant_content_from_easy(&message.content) {
                messages.push(assistant_message(Some(content), None, None));
            }
        }
    }
    Ok(())
}

fn messages_from_item(
    item: &responses::Item,
    messages: &mut Vec<chat::ChatCompletionRequestMessage>,
) -> TranslationResult<()> {
    match item {
        responses::Item::Message(message) => match message {
            responses::MessageItem::Output(output) => {
                push_assistant_output_message(output, messages);
            }
            responses::MessageItem::Input(input) => {
                push_input_message(input, messages);
            }
        },
        responses::Item::FunctionCall(call) => {
            push_assistant_tool_call(messages, call.into());
        }
        responses::Item::FunctionCallOutput(output) => {
            messages.push(tool_message(
                (&output.output).into(),
                output.call_id.clone(),
            ));
        }
        responses::Item::CustomToolCall(call) => {
            push_assistant_tool_call(messages, call.into());
        }
        responses::Item::CustomToolCallOutput(output) => {
            messages.push(tool_message(
                chat::ChatCompletionRequestToolMessageContent::Text(custom_tool_output_to_string(
                    &output.output,
                )),
                output.call_id.clone(),
            ));
        }
        responses::Item::Reasoning(reasoning) => {
            push_assistant_reasoning(reasoning, messages);
        }
        other => {
            // Responses items without a Chat Completions representation
            // (reasoning, hosted tool calls, MCP, search, etc.) are surfaced as
            // a user text marker so the omission is observable rather than
            // silently dropped.
            messages.push(user_text_message(format!(
                "[OpenAI Responses item `{}` omitted during Chat Completions translation]",
                other.as_ref()
            )));
        }
    }
    Ok(())
}

fn push_assistant_output_message(
    output: &responses::OutputMessage,
    messages: &mut Vec<chat::ChatCompletionRequestMessage>,
) {
    let content_parts: Vec<responses::OutputMessageContent> = output.content.clone();
    let mut chat_content: Vec<chat::ChatCompletionRequestAssistantMessageContentPart> = Vec::new();
    for part in content_parts {
        match part {
            responses::OutputMessageContent::OutputText(text) => {
                chat_content.push(
                    chat::ChatCompletionRequestAssistantMessageContentPart::Text(text_part(
                        text.text,
                    )),
                );
            }
            responses::OutputMessageContent::Refusal(refusal) => {
                chat_content.push(
                    chat::ChatCompletionRequestAssistantMessageContentPart::Refusal(
                        chat::ChatCompletionRequestMessageContentPartRefusal {
                            refusal: refusal.refusal,
                        },
                    ),
                );
            }
        }
    }

    if chat_content.is_empty() {
        // Skip empty assistant output messages; Chat assistant messages require
        // at least content or tool_calls.
        return;
    }

    if let Some(chat::ChatCompletionRequestMessage::Assistant(assistant)) = messages.last_mut()
        && assistant.content.is_none()
        && assistant.refusal.is_none()
        && assistant.tool_calls.is_none()
        && assistant.reasoning_content.is_some()
    {
        assistant.content = Some(chat::ChatCompletionRequestAssistantMessageContent::Array(
            chat_content,
        ));
        return;
    }

    messages.push(assistant_message(
        Some(chat::ChatCompletionRequestAssistantMessageContent::Array(
            chat_content,
        )),
        None,
        None,
    ));
}

fn push_assistant_reasoning(
    reasoning: &responses::ReasoningItem,
    messages: &mut Vec<chat::ChatCompletionRequestMessage>,
) {
    let mut parts: Vec<String> = reasoning
        .summary
        .iter()
        .map(|part| match part {
            responses::SummaryPart::SummaryText(text) => text.text.clone(),
        })
        .collect();
    if let Some(content) = reasoning.content.as_ref() {
        parts.extend(content.iter().map(|part| match part {
            responses::ReasoningItemContent::ReasoningText(text) => text.text.clone(),
        }));
    }
    let reasoning_content = parts.concat();
    if reasoning_content.is_empty() {
        tracing::trace!(
            encrypted = reasoning.encrypted_content.is_some(),
            reason = "Chat reasoning_content requires visible reasoning text",
            "skipping Responses reasoning item during Chat Completions request translation"
        );
        return;
    }

    if let Some(chat::ChatCompletionRequestMessage::Assistant(assistant)) = messages.last_mut() {
        assistant
            .reasoning_content
            .get_or_insert_with(String::new)
            .push_str(&reasoning_content);
        return;
    }

    messages.push(assistant_message(None, Some(reasoning_content), None));
}

fn push_input_message(
    input: &responses::InputMessage,
    messages: &mut Vec<chat::ChatCompletionRequestMessage>,
) {
    match input.role {
        responses::InputRole::System => {
            if let Some(text) = join_input_text(&input.content)
                && !text.is_empty()
            {
                messages.push(system_text_message(text));
            }
        }
        responses::InputRole::Developer => {
            if let Some(text) = join_input_text(&input.content)
                && !text.is_empty()
            {
                messages.push(developer_text_message(text));
            }
        }
        responses::InputRole::User => {
            let parts: Vec<chat::ChatCompletionRequestUserMessageContentPart> = input
                .content
                .iter()
                .filter_map(|content| match content {
                    responses::InputContent::InputText(text) => {
                        Some(chat::ChatCompletionRequestUserMessageContentPart::Text(
                            text_part(text.text.clone()),
                        ))
                    }
                    responses::InputContent::InputImage(image) => {
                        let Some(url) = image.image_url.clone() else {
                            tracing::trace!(
                                source_field = "input[].content[].image_url",
                                reason = "Chat Completions user image parts require an image URL",
                                "skipping Responses input content during Chat Completions translation"
                            );
                            return None;
                        };
                        Some(chat::ChatCompletionRequestUserMessageContentPart::ImageUrl(
                            chat::ChatCompletionRequestMessageContentPartImage {
                                image_url: chat::ImageUrl {
                                    url,
                                    detail: Some(image.detail.into()),
                                },
                            },
                        ))
                    }
                    responses::InputContent::InputFile(_) => {
                        tracing::trace!(
                            source_field = "input[].content[].input_file",
                            reason = "Chat Completions user message content has no compatible hosted file representation",
                            "skipping Responses input content during Chat Completions translation"
                        );
                        None
                    }
                })
                .collect();

            let content = if parts.is_empty() {
                chat::ChatCompletionRequestUserMessageContent::Text(String::new())
            } else {
                chat::ChatCompletionRequestUserMessageContent::Array(parts)
            };
            messages.push(user_message(content));
        }
    }
}

fn push_assistant_tool_call(
    messages: &mut Vec<chat::ChatCompletionRequestMessage>,
    tool_call: chat::ChatCompletionMessageToolCalls,
) {
    if let Some(chat::ChatCompletionRequestMessage::Assistant(assistant)) = messages.last_mut() {
        assistant
            .tool_calls
            .get_or_insert_with(Vec::new)
            .push(tool_call);
        return;
    }
    messages.push(assistant_message(None, None, Some(vec![tool_call])));
}

fn text_parts_from_easy_content(content: &responses::EasyInputContent) -> Vec<String> {
    match content {
        responses::EasyInputContent::Text(text) => vec![text.clone()],
        responses::EasyInputContent::ContentList(parts) => parts
            .iter()
            .filter_map(|part| match part {
                responses::InputContent::InputText(text) => Some(text.text.clone()),
                other => {
                    tracing::trace!(
                        discriminant = ?std::mem::discriminant(other),
                        reason = "instruction messages can only be represented as Chat text",
                        "skipping Responses instruction content during Chat Completions translation"
                    );
                    None
                }
            })
            .collect(),
    }
}

fn join_input_text(content: &[responses::InputContent]) -> Option<String> {
    let parts: Vec<String> = content
        .iter()
        .filter_map(|part| match part {
            responses::InputContent::InputText(text) => Some(text.text.clone()),
            other => {
                tracing::trace!(
                    discriminant = ?std::mem::discriminant(other),
                    reason = "instruction messages can only be represented as Chat text",
                    "skipping Responses instruction content during Chat Completions translation"
                );
                None
            }
        })
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(""))
    }
}

fn assistant_content_from_easy(
    content: &responses::EasyInputContent,
) -> Option<chat::ChatCompletionRequestAssistantMessageContent> {
    match content {
        responses::EasyInputContent::Text(text) if !text.is_empty() => Some(
            chat::ChatCompletionRequestAssistantMessageContent::Text(text.clone()),
        ),
        responses::EasyInputContent::Text(_) => None,
        responses::EasyInputContent::ContentList(parts) => {
            let mut translated = Vec::new();
            for part in parts {
                if let responses::InputContent::InputText(text) = part {
                    translated.push(
                        chat::ChatCompletionRequestAssistantMessageContentPart::Text(text_part(
                            text.text.clone(),
                        )),
                    );
                } else {
                    tracing::trace!(
                        discriminant = ?std::mem::discriminant(part),
                        reason = "Chat assistant history content can only represent text here",
                        "skipping Responses assistant content during Chat Completions translation"
                    );
                }
            }
            if translated.is_empty() {
                None
            } else {
                Some(chat::ChatCompletionRequestAssistantMessageContent::Array(
                    translated,
                ))
            }
        }
    }
}

fn custom_tool_output_to_string(output: &responses::CustomToolCallOutputOutput) -> String {
    match output {
        responses::CustomToolCallOutputOutput::Text(text) => text.clone(),
        responses::CustomToolCallOutputOutput::List(list) => {
            serde_json::to_string(list).unwrap_or_default()
        }
    }
}

fn is_non_instruction_message(message: &chat::ChatCompletionRequestMessage) -> bool {
    !matches!(
        message,
        chat::ChatCompletionRequestMessage::System(_)
            | chat::ChatCompletionRequestMessage::Developer(_)
    )
}

impl TryFrom<&responses::EasyInputContent> for chat::ChatCompletionRequestUserMessageContent {
    type Error = TranslationError;

    fn try_from(
        content: &responses::EasyInputContent,
    ) -> TranslationResult<chat::ChatCompletionRequestUserMessageContent> {
        match content {
            responses::EasyInputContent::Text(text) => Ok(Self::Text(text.clone())),
            responses::EasyInputContent::ContentList(parts) => {
                let translated = parts
                    .iter()
                    .map(chat::ChatCompletionRequestUserMessageContentPart::try_from)
                    .collect::<TranslationResult<Vec<_>>>()?;
                Ok(Self::Array(translated))
            }
        }
    }
}

impl TryFrom<&responses::InputContent> for chat::ChatCompletionRequestUserMessageContentPart {
    type Error = TranslationError;

    fn try_from(
        content: &responses::InputContent,
    ) -> TranslationResult<chat::ChatCompletionRequestUserMessageContentPart> {
        match content {
            responses::InputContent::InputText(text) => {
                if text.text.is_empty() {
                    return Err(TranslationError::InvalidPayload(
                        "OpenAI Responses input_text content part cannot be empty when translating to Chat Completions"
                            .to_string(),
                    ));
                }
                Ok(Self::Text(text_part(text.text.clone())))
            }
            responses::InputContent::InputImage(image) => {
                let url = image.image_url.clone().ok_or_else(|| {
                    TranslationError::InvalidPayload(
                        "OpenAI Responses input_image content without image_url cannot be translated to Chat Completions"
                            .to_string(),
                    )
                })?;
                Ok(Self::ImageUrl(chat::ChatCompletionRequestMessageContentPartImage {
                    image_url: chat::ImageUrl {
                        url,
                        detail: Some(image.detail.into()),
                    },
                }))
            }
            responses::InputContent::InputFile(_) => Err(TranslationError::InvalidPayload(
                "OpenAI Responses input_file content cannot be translated to Chat Completions user content"
                    .to_string(),
            )),
        }
    }
}

impl From<&responses::FunctionCallOutput> for chat::ChatCompletionRequestToolMessageContent {
    fn from(output: &responses::FunctionCallOutput) -> Self {
        match output {
            responses::FunctionCallOutput::Text(text) => Self::Text(text.clone()),
            responses::FunctionCallOutput::Content(parts) => {
                let translated = parts
                    .iter()
                    .filter_map(|part| match part {
                        responses::InputContent::InputText(text) => {
                            Some(text_part(text.text.clone()))
                        }
                        other => {
                            tracing::trace!(
                                discriminant = ?std::mem::discriminant(other),
                                reason = "Chat tool messages can only represent text output parts",
                                "skipping Responses function output content during Chat Completions translation"
                            );
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                if translated.is_empty() {
                    Self::Text(String::new())
                } else {
                    Self::Array(
                        translated
                            .into_iter()
                            .map(chat::ChatCompletionRequestToolMessageContentPart::Text)
                            .collect(),
                    )
                }
            }
        }
    }
}
