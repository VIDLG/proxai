use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::chat_completions::request::wire as chat_request;
use crate::protocol::openai::responses;
use crate::translation::TranslationScope;
use crate::translation::openai_chat_completions::outbound::{
    text_part, user_file_part, user_image_url_part,
};

pub(super) fn system_content_from_easy_input(
    content: &responses::EasyInputContent,
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionRequestSystemMessageContent> {
    match content {
        responses::EasyInputContent::Text(text) => (!text.trim().is_empty())
            .then(|| chat::ChatCompletionRequestSystemMessageContent::Text(text.clone())),
        responses::EasyInputContent::ContentList(parts) => system_content_from_input(parts, scope),
    }
}

pub(super) fn developer_content_from_easy_input(
    content: &responses::EasyInputContent,
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionRequestDeveloperMessageContent> {
    match content {
        responses::EasyInputContent::Text(text) => (!text.trim().is_empty())
            .then(|| chat::ChatCompletionRequestDeveloperMessageContent::Text(text.clone())),
        responses::EasyInputContent::ContentList(parts) => {
            developer_content_from_input(parts, scope)
        }
    }
}

pub(super) fn system_content_from_input(
    content: &[responses::InputContent],
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionRequestSystemMessageContent> {
    let parts = instruction_text_parts_from_input(content, scope);
    match parts.as_slice() {
        [text] if text.prompt_cache_breakpoint.is_none() => Some(
            chat::ChatCompletionRequestSystemMessageContent::Text(text.text.clone()),
        ),
        [] => None,
        _ => Some(chat::ChatCompletionRequestSystemMessageContent::Array(
            parts
                .into_iter()
                .map(chat::ChatCompletionRequestSystemMessageContentPart::Text)
                .collect(),
        )),
    }
}

pub(super) fn developer_content_from_input(
    content: &[responses::InputContent],
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionRequestDeveloperMessageContent> {
    let parts = instruction_text_parts_from_input(content, scope);
    match parts.as_slice() {
        [text] if text.prompt_cache_breakpoint.is_none() => Some(
            chat::ChatCompletionRequestDeveloperMessageContent::Text(text.text.clone()),
        ),
        [] => None,
        _ => Some(chat::ChatCompletionRequestDeveloperMessageContent::Array(
            parts
                .into_iter()
                .map(chat::ChatCompletionRequestDeveloperMessageContentPart::Text)
                .collect(),
        )),
    }
}

pub(super) fn user_content_from_easy_input(
    content: &responses::EasyInputContent,
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionRequestUserMessageContent> {
    match content {
        responses::EasyInputContent::Text(text) => Some(
            chat::ChatCompletionRequestUserMessageContent::Text(text.clone()),
        ),
        responses::EasyInputContent::ContentList(parts) => user_content_from_input(parts, scope),
    }
}

pub(super) fn user_content_from_input(
    content: &[responses::InputContent],
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionRequestUserMessageContent> {
    let parts = content
        .iter()
        .filter_map(|part| user_content_part_from_input(part, scope))
        .collect::<Vec<_>>();

    match parts.as_slice() {
        [chat::ChatCompletionRequestUserMessageContentPart::Text(text)]
            if text.prompt_cache_breakpoint.is_none() =>
        {
            Some(chat::ChatCompletionRequestUserMessageContent::Text(
                text.text.clone(),
            ))
        }
        [] => None,
        _ => Some(chat::ChatCompletionRequestUserMessageContent::Array(parts)),
    }
}

pub(super) fn assistant_content_from_easy_input(
    content: &responses::EasyInputContent,
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionRequestAssistantMessageContent> {
    match content {
        responses::EasyInputContent::Text(text) => (!text.is_empty())
            .then(|| chat::ChatCompletionRequestAssistantMessageContent::Text(text.clone())),
        responses::EasyInputContent::ContentList(parts) => {
            assistant_content_from_input(parts, scope)
        }
    }
}

fn assistant_content_from_input(
    content: &[responses::InputContent],
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionRequestAssistantMessageContent> {
    assistant_content_from_parts(
        content
            .iter()
            .filter_map(|part| assistant_content_part_from_input(part, scope))
            .collect(),
    )
}

pub(super) fn assistant_content_from_output(
    output: &responses::OutputMessage,
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionRequestAssistantMessageContent> {
    assistant_content_from_parts(
        output
            .content
            .iter()
            .map(|part| match part {
                responses::OutputMessageContent::OutputText(text) => {
                    if !text.annotations.is_empty() {
                        scope.dropped(
                            "Responses output_text annotations",
                            "Chat assistant history content cannot represent output text annotations",
                        );
                    }
                    if !text.logprobs.is_empty() {
                        scope.dropped(
                            "Responses output_text logprobs",
                            "Chat assistant history content cannot represent output token log probabilities",
                        );
                    }
                    chat::ChatCompletionRequestAssistantMessageContentPart::Text(text_part(
                        text.text.clone(),
                    ))
                }
                responses::OutputMessageContent::Refusal(refusal) => {
                    chat::ChatCompletionRequestAssistantMessageContentPart::Refusal(
                        chat::ChatCompletionRequestMessageContentPartRefusal {
                            refusal: refusal.refusal.clone(),
                        },
                    )
                }
            })
            .collect(),
    )
}

pub(super) fn assistant_reasoning_content_from_item(
    reasoning: &responses::ReasoningItem,
    scope: &TranslationScope,
) -> Option<String> {
    let mut parts = reasoning
        .summary
        .iter()
        .map(|part| match part {
            responses::SummaryPart::SummaryText(text) => text.text.as_str(),
        })
        .collect::<Vec<_>>();
    if let Some(content) = reasoning.content.as_ref() {
        parts.extend(content.iter().map(|part| match part {
            responses::ReasoningItemContent::ReasoningText(text) => text.text.as_str(),
        }));
    }
    let content = parts.concat();
    if content.is_empty() {
        scope.dropped(
            "Responses reasoning item",
            if reasoning.encrypted_content.is_non_null() {
                "Chat reasoning_content cannot represent encrypted reasoning without visible text"
            } else {
                "Chat reasoning_content requires visible reasoning text"
            },
        );
        return None;
    }
    if reasoning.encrypted_content.is_non_null() {
        scope.dropped(
            "Responses reasoning encrypted_content",
            "Chat reasoning_content preserves visible reasoning text but cannot represent encrypted reasoning state",
        );
    }
    Some(content)
}

fn assistant_content_from_parts(
    parts: Vec<chat::ChatCompletionRequestAssistantMessageContentPart>,
) -> Option<chat::ChatCompletionRequestAssistantMessageContent> {
    match parts.as_slice() {
        [chat::ChatCompletionRequestAssistantMessageContentPart::Text(text)]
            if text.prompt_cache_breakpoint.is_none() =>
        {
            Some(chat::ChatCompletionRequestAssistantMessageContent::Text(
                text.text.clone(),
            ))
        }
        [] => None,
        _ => Some(chat::ChatCompletionRequestAssistantMessageContent::Array(
            parts,
        )),
    }
}

pub(super) fn tool_content_from_function_output(
    output: &responses::FunctionCallOutput,
    scope: &TranslationScope,
) -> chat::ChatCompletionRequestToolMessageContent {
    match output {
        responses::FunctionCallOutput::Text(text) => {
            chat::ChatCompletionRequestToolMessageContent::Text(text.clone())
        }
        responses::FunctionCallOutput::Content(parts) => {
            tool_content_from_input(parts, "Responses function_call_output", scope)
        }
    }
}

pub(super) fn tool_content_from_custom_output(
    output: &responses::CustomToolCallOutputOutput,
    scope: &TranslationScope,
) -> chat::ChatCompletionRequestToolMessageContent {
    match output {
        responses::CustomToolCallOutputOutput::Text(text) => {
            chat::ChatCompletionRequestToolMessageContent::Text(text.clone())
        }
        responses::CustomToolCallOutputOutput::List(parts) => {
            tool_content_from_input(parts, "Responses custom_tool_call_output", scope)
        }
    }
}

fn tool_content_from_input(
    content: &[responses::InputContent],
    source: &'static str,
    scope: &TranslationScope,
) -> chat::ChatCompletionRequestToolMessageContent {
    let parts = content
        .iter()
        .filter_map(|part| tool_content_part_from_input(part, source, scope))
        .collect::<Vec<_>>();
    match parts.as_slice() {
        [chat::ChatCompletionRequestToolMessageContentPart::Text(text)]
            if text.prompt_cache_breakpoint.is_none() =>
        {
            chat::ChatCompletionRequestToolMessageContent::Text(text.text.clone())
        }
        [] => chat::ChatCompletionRequestToolMessageContent::Text(String::new()),
        _ => chat::ChatCompletionRequestToolMessageContent::Array(parts),
    }
}

fn instruction_text_parts_from_input(
    content: &[responses::InputContent],
    scope: &TranslationScope,
) -> Vec<chat_request::ChatCompletionRequestMessageContentPartText> {
    content
        .iter()
        .filter_map(|part| instruction_text_part_from_input(part, scope))
        .collect()
}

fn instruction_text_part_from_input(
    content: &responses::InputContent,
    scope: &TranslationScope,
) -> Option<chat_request::ChatCompletionRequestMessageContentPartText> {
    match content {
        responses::InputContent::InputText(text) if !text.text.trim().is_empty() => {
            Some(text.into())
        }
        responses::InputContent::InputText(_) => None,
        responses::InputContent::InputImage(_) => {
            scope.dropped(
                "Responses instruction content `input_image`",
                "Chat instruction messages can only represent text",
            );
            None
        }
        responses::InputContent::InputFile(_) => {
            scope.dropped(
                "Responses instruction content `input_file`",
                "Chat instruction messages can only represent text",
            );
            None
        }
    }
}

fn assistant_content_part_from_input(
    content: &responses::InputContent,
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionRequestAssistantMessageContentPart> {
    // Chat assistant request content supports text/refusal parts only. Responses
    // `InputContent` has no refusal variant, while image/file parts are valid only
    // on Chat user messages. An empty easy-assistant text has no tool-call fallback,
    // so omit that empty turn rather than generate a contentless assistant message.
    match content {
        responses::InputContent::InputText(text) if !text.text.is_empty() => {
            Some(chat::ChatCompletionRequestAssistantMessageContentPart::Text(text.into()))
        }
        responses::InputContent::InputText(_) => None,
        responses::InputContent::InputImage(_) => {
            scope.dropped(
                "Responses assistant content `input_image`",
                "Chat assistant history content can only represent text",
            );
            None
        }
        responses::InputContent::InputFile(_) => {
            scope.dropped(
                "Responses assistant content `input_file`",
                "Chat assistant history content can only represent text",
            );
            None
        }
    }
}

fn tool_content_part_from_input(
    content: &responses::InputContent,
    source: &'static str,
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionRequestToolMessageContentPart> {
    match content {
        responses::InputContent::InputText(text) => Some(
            chat::ChatCompletionRequestToolMessageContentPart::Text(text.into()),
        ),
        responses::InputContent::InputImage(_) => {
            scope.dropped(
                format!("{source} `input_image`"),
                "Chat tool messages can only represent text output content",
            );
            None
        }
        responses::InputContent::InputFile(_) => {
            scope.dropped(
                format!("{source} `input_file`"),
                "Chat tool messages can only represent text output content",
            );
            None
        }
    }
}

fn user_content_part_from_input(
    content: &responses::InputContent,
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionRequestUserMessageContentPart> {
    match content {
        responses::InputContent::InputText(text) => Some(
            chat::ChatCompletionRequestUserMessageContentPart::Text(text.into()),
        ),
        responses::InputContent::InputImage(image) => {
            let prompt_cache_breakpoint = image.prompt_cache_breakpoint.as_ref().map(Into::into);
            if let Some(url) = image.image_url.as_non_null().cloned() {
                if image.file_id.is_non_null() {
                    scope.dropped(
                        "Responses input_image.file_id",
                        "Responses input_image also supplied image_url; Chat image content preserves image_url and omits the alternate file_id",
                    );
                }
                if image.detail == responses::ImageDetail::Original {
                    scope.adapted(
                        "Responses input_image detail `original`",
                        "Chat Completions has no `original` image detail; falling back to `auto` while preserving the image",
                    );
                }
                return Some(user_image_url_part(
                    url,
                    Some(image.detail.into()),
                    prompt_cache_breakpoint,
                ));
            }

            if let Some(file_id) = image.file_id.as_non_null().cloned() {
                scope.adapted(
                    "Responses input_image.file_id",
                    "Chat image content accepts only image_url; projecting the uploaded image as Chat file content",
                );
                if image.detail != responses::ImageDetail::Auto {
                    scope.dropped(
                        format!("Responses input_image detail `{}`", image.detail),
                        "Chat file content has no image rendering detail control",
                    );
                }
                return Some(user_file_part(
                    chat::FileObject {
                        filename: None,
                        file_data: None,
                        file_id: Some(file_id),
                    },
                    prompt_cache_breakpoint,
                ));
            }

            scope.dropped(
                "Responses input_image content",
                "Chat image content requires image_url or file_id",
            );
            None
        }
        responses::InputContent::InputFile(file) => {
            if file.file_url.is_some() {
                scope.dropped(
                    "Responses input_file.file_url",
                    "Chat file content supports file_id or file_data, not file_url",
                );
            }
            if file.detail.is_some() {
                scope.dropped(
                    "Responses input_file.detail",
                    "Chat file content has no document rendering detail control",
                );
            }
            let file_id = file.file_id.as_non_null().cloned();
            if file_id.is_none() && file.file_data.is_none() {
                scope.dropped(
                    "Responses input_file content",
                    "Chat file content requires file_id or file_data",
                );
                return None;
            }
            Some(user_file_part(
                chat::FileObject {
                    filename: file.filename.clone(),
                    file_data: file.file_data.clone(),
                    file_id,
                },
                file.prompt_cache_breakpoint.as_ref().map(Into::into),
            ))
        }
    }
}
