use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::chat_completions::request::wire as chat_request;
use crate::translation::{TranslationError, TranslationResult};

use super::types::non_empty;

pub(super) fn chat_messages(
    message: anthropic::MessageParam,
) -> TranslationResult<Vec<chat::ChatCompletionRequestMessage>> {
    Ok(match (message.role, message.content) {
        (anthropic::Role::User, anthropic::MessageParamContent::Text(text)) => {
            vec![chat::ChatCompletionRequestMessage::User(
                chat::ChatCompletionRequestUserMessage {
                    content: chat::ChatCompletionRequestUserMessageContent::Text(text),
                    name: None,
                },
            )]
        }
        (anthropic::Role::User, anthropic::MessageParamContent::Blocks(blocks)) => {
            user_block_messages(blocks)?
        }
        (anthropic::Role::Assistant, anthropic::MessageParamContent::Text(text)) => {
            vec![chat::ChatCompletionRequestMessage::Assistant(
                chat::ChatCompletionRequestAssistantMessage {
                    content: Some(chat::ChatCompletionRequestAssistantMessageContent::Text(
                        text,
                    )),
                    refusal: None,
                    name: None,
                    audio: None,
                    tool_calls: None,
                },
            )]
        }
        (anthropic::Role::Assistant, anthropic::MessageParamContent::Blocks(blocks)) => {
            vec![assistant_blocks_message(blocks)?]
        }
        (anthropic::Role::System, anthropic::MessageParamContent::Text(text)) => {
            vec![chat::ChatCompletionRequestSystemMessageContent::Text(text).into()]
        }
        (anthropic::Role::System, anthropic::MessageParamContent::Blocks(blocks)) => {
            vec![system_blocks_message(blocks)?]
        }
    })
}

// User messages -------------------------------------------------------------

fn user_block_messages(
    blocks: Vec<anthropic::ContentBlockParam>,
) -> TranslationResult<Vec<chat::ChatCompletionRequestMessage>> {
    let mut chat_messages = Vec::new();
    let mut pending_user_parts = Vec::new();

    for block in blocks {
        match block {
            anthropic::ContentBlockParam::ToolResult(block) => {
                if let Some(message) = take_pending_user_message(&mut pending_user_parts) {
                    chat_messages.push(message);
                }
                chat_messages.push(block.try_into()?);
            }
            block => match block.try_into()? {
                chat::ChatCompletionRequestUserMessageContent::Text(text) => {
                    pending_user_parts.push(text.into());
                }
                chat::ChatCompletionRequestUserMessageContent::Array(parts) => {
                    pending_user_parts.extend(parts);
                }
            },
        }
    }

    if let Some(message) = take_pending_user_message(&mut pending_user_parts) {
        chat_messages.push(message);
    }

    if chat_messages.is_empty() {
        chat_messages
            .push(chat::ChatCompletionRequestUserMessageContent::Text(String::new()).into());
    }

    Ok(chat_messages)
}

fn take_pending_user_message(
    pending_user_parts: &mut Vec<chat::ChatCompletionRequestUserMessageContentPart>,
) -> Option<chat::ChatCompletionRequestMessage> {
    (!pending_user_parts.is_empty()).then(|| {
        chat::ChatCompletionRequestUserMessageContent::from(std::mem::take(pending_user_parts))
            .into()
    })
}

impl From<chat::ChatCompletionRequestUserMessageContent> for chat::ChatCompletionRequestMessage {
    fn from(content: chat::ChatCompletionRequestUserMessageContent) -> Self {
        Self::User(chat::ChatCompletionRequestUserMessage {
            content,
            name: None,
        })
    }
}

impl From<Vec<chat::ChatCompletionRequestUserMessageContentPart>>
    for chat::ChatCompletionRequestUserMessageContent
{
    fn from(content_parts: Vec<chat::ChatCompletionRequestUserMessageContentPart>) -> Self {
        match content_parts.as_slice() {
            [chat::ChatCompletionRequestUserMessageContentPart::Text(part)] => {
                Self::Text(part.text.clone())
            }
            _ => Self::Array(content_parts),
        }
    }
}

impl From<String> for chat::ChatCompletionRequestUserMessageContentPart {
    fn from(text: String) -> Self {
        Self::Text(text.into())
    }
}

impl TryFrom<anthropic::ContentBlockParam> for chat::ChatCompletionRequestUserMessageContent {
    type Error = TranslationError;

    fn try_from(block: anthropic::ContentBlockParam) -> TranslationResult<Self> {
        match block {
            anthropic::ContentBlockParam::Text(block) => Ok(Self::Text(block.text)),
            anthropic::ContentBlockParam::Image(block) => Ok(Self::Array(vec![block.into()])),
            anthropic::ContentBlockParam::Document(block) => block.try_into(),
            anthropic::ContentBlockParam::ContainerUpload(block) => {
                Ok(Self::Array(vec![block.into()]))
            }
            other => Err(TranslationError::InvalidPayload(format!(
                "Anthropic user content block `{}` cannot be translated to Chat Completions request content",
                other.as_ref()
            ))),
        }
    }
}

// Assistant messages --------------------------------------------------------

fn assistant_blocks_message(
    blocks: Vec<anthropic::ContentBlockParam>,
) -> TranslationResult<chat::ChatCompletionRequestMessage> {
    let mut content_parts = Vec::new();
    let mut tool_calls = Vec::new();

    let mut seen_tool_use = false;

    for block in blocks {
        match block.try_into()? {
            AssistantBlock::Text(text) => {
                if seen_tool_use {
                    return Err(TranslationError::InvalidPayload(
                        "Anthropic assistant text blocks after tool_use blocks cannot be translated to Chat Completions; Chat assistant messages cannot preserve text/tool_call ordering"
                            .to_string(),
                    ));
                }
                content_parts.push(text.into());
            }
            AssistantBlock::ToolCall(tool_call) => {
                seen_tool_use = true;
                tool_calls.push(tool_call);
            }
        }
    }

    Ok(chat::ChatCompletionRequestMessage::Assistant(
        chat::ChatCompletionRequestAssistantMessage {
            content: non_empty(content_parts).map(Into::into),
            refusal: None,
            name: None,
            audio: None,
            tool_calls: non_empty(tool_calls),
        },
    ))
}

enum AssistantBlock {
    Text(String),
    ToolCall(chat::ChatCompletionMessageToolCalls),
}

impl TryFrom<anthropic::ContentBlockParam> for AssistantBlock {
    type Error = TranslationError;

    fn try_from(block: anthropic::ContentBlockParam) -> TranslationResult<Self> {
        match block {
            anthropic::ContentBlockParam::Text(block) => Ok(Self::Text(block.text)),
            anthropic::ContentBlockParam::ToolUse(block) => Ok(Self::ToolCall(block.try_into()?)),
            other => Err(TranslationError::InvalidPayload(format!(
                "Anthropic assistant content block `{}` cannot be translated to Chat Completions request content",
                other.as_ref()
            ))),
        }
    }
}

impl From<Vec<chat::ChatCompletionRequestAssistantMessageContentPart>>
    for chat::ChatCompletionRequestAssistantMessageContent
{
    fn from(content_parts: Vec<chat::ChatCompletionRequestAssistantMessageContentPart>) -> Self {
        match content_parts.as_slice() {
            [chat::ChatCompletionRequestAssistantMessageContentPart::Text(part)] => {
                Self::Text(part.text.clone())
            }
            _ => Self::Array(content_parts),
        }
    }
}

impl From<String> for chat::ChatCompletionRequestAssistantMessageContentPart {
    fn from(text: String) -> Self {
        Self::Text(text.into())
    }
}

impl TryFrom<anthropic::ToolUseBlockParam> for chat::ChatCompletionMessageToolCalls {
    type Error = TranslationError;

    fn try_from(block: anthropic::ToolUseBlockParam) -> TranslationResult<Self> {
        Ok(Self::Function(chat::ChatCompletionMessageToolCall {
            id: block.id,
            function: chat::FunctionCall {
                name: block.name,
                arguments: serde_json::to_string(&block.input)?,
            },
        }))
    }
}

// System messages -----------------------------------------------------------

fn system_blocks_message(
    blocks: Vec<anthropic::ContentBlockParam>,
) -> TranslationResult<chat::ChatCompletionRequestMessage> {
    let mut content_parts = Vec::new();

    for block in blocks {
        match block {
            anthropic::ContentBlockParam::Text(block) => content_parts.push(block.into()),
            anthropic::ContentBlockParam::MidConversationSystem(block) => {
                content_parts.extend(block.content.into_iter().map(Into::into));
            }
            other => {
                return Err(TranslationError::InvalidPayload(format!(
                    "Anthropic system content block `{}` cannot be translated to Chat Completions system message content",
                    other.as_ref()
                )));
            }
        }
    }

    Ok(chat::ChatCompletionRequestSystemMessageContent::from(content_parts).into())
}

impl From<anthropic::SystemPrompt> for chat::ChatCompletionRequestMessage {
    fn from(system: anthropic::SystemPrompt) -> Self {
        Self::System(chat::ChatCompletionRequestSystemMessage {
            content: system.into(),
            name: None,
        })
    }
}

impl From<chat::ChatCompletionRequestSystemMessageContent> for chat::ChatCompletionRequestMessage {
    fn from(content: chat::ChatCompletionRequestSystemMessageContent) -> Self {
        Self::System(chat::ChatCompletionRequestSystemMessage {
            content,
            name: None,
        })
    }
}

impl From<Vec<chat::ChatCompletionRequestSystemMessageContentPart>>
    for chat::ChatCompletionRequestSystemMessageContent
{
    fn from(content_parts: Vec<chat::ChatCompletionRequestSystemMessageContentPart>) -> Self {
        match content_parts.as_slice() {
            [chat::ChatCompletionRequestSystemMessageContentPart::Text(part)] => {
                Self::Text(part.text.clone())
            }
            _ => Self::Array(content_parts),
        }
    }
}

impl From<anthropic::SystemPrompt> for chat::ChatCompletionRequestSystemMessageContent {
    fn from(system: anthropic::SystemPrompt) -> Self {
        match system {
            anthropic::SystemPrompt::Text(text) => Self::Text(text),
            anthropic::SystemPrompt::Blocks(blocks) => blocks
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .into(),
        }
    }
}

impl From<anthropic::TypedTextBlockParam> for chat::ChatCompletionRequestSystemMessageContentPart {
    fn from(block: anthropic::TypedTextBlockParam) -> Self {
        // Chat request system text parts have no citation field;
        // Anthropic system block citations cannot be represented here.
        Self::Text(block.text.into())
    }
}

impl From<anthropic::TextBlockParam> for chat::ChatCompletionRequestSystemMessageContentPart {
    fn from(block: anthropic::TextBlockParam) -> Self {
        // Chat request system text parts have no citation field;
        // Anthropic system block citations cannot be represented here.
        Self::Text(block.text.into())
    }
}

// Shared user/system text parts --------------------------------------------

impl From<String> for chat_request::ChatCompletionRequestMessageContentPartText {
    fn from(text: String) -> Self {
        Self { text }
    }
}

impl From<anthropic::TextBlockParam> for chat::ChatCompletionRequestUserMessageContentPart {
    fn from(block: anthropic::TextBlockParam) -> Self {
        // Chat request text parts have no citation field; Anthropic text block
        // citations cannot be represented here.
        Self::Text(block.text.into())
    }
}

// Documents, files, and images ---------------------------------------------

fn base64_pdf_file_part(
    source: anthropic::Base64PdfSource,
    filename: Option<String>,
) -> chat::ChatCompletionRequestUserMessageContentPart {
    chat::ChatCompletionRequestUserMessageContentPart::File(
        chat::ChatCompletionRequestMessageContentPartFile {
            file: chat::FileObject {
                file_data: Some(source.data),
                file_id: None,
                filename,
            },
        },
    )
}

impl From<anthropic::ContainerUploadBlockParam>
    for chat::ChatCompletionRequestUserMessageContentPart
{
    fn from(block: anthropic::ContainerUploadBlockParam) -> Self {
        Self::File(chat::ChatCompletionRequestMessageContentPartFile {
            file: chat::FileObject {
                file_data: None,
                file_id: Some(block.file_id),
                filename: None,
            },
        })
    }
}

impl TryFrom<anthropic::DocumentBlockParam> for chat::ChatCompletionRequestUserMessageContent {
    type Error = TranslationError;

    fn try_from(block: anthropic::DocumentBlockParam) -> TranslationResult<Self> {
        match block.source {
            anthropic::DocumentBlockParamSource::Base64(source) => Ok(Self::Array(vec![
                base64_pdf_file_part(source, block.title),
            ])),
            anthropic::DocumentBlockParamSource::PlainText(source) => Ok(Self::Text(source.data)),
            anthropic::DocumentBlockParamSource::Content(source) => Ok(source.into()),
            anthropic::DocumentBlockParamSource::Url(_) => Err(TranslationError::InvalidPayload(
                "Anthropic document URL blocks cannot be translated to Chat Completions request content; upload the file or use base64/text document content"
                    .to_string(),
            )),
        }
    }
}

impl From<anthropic::ContentBlockSource> for chat::ChatCompletionRequestUserMessageContent {
    fn from(source: anthropic::ContentBlockSource) -> Self {
        match source.content {
            anthropic::ContentBlockSourceContentUnion::Text(text) => Self::Text(text),
            anthropic::ContentBlockSourceContentUnion::Blocks(blocks) => {
                Self::Array(blocks.into_iter().map(Into::into).collect())
            }
        }
    }
}

impl From<anthropic::ContentBlockSourceContent>
    for chat::ChatCompletionRequestUserMessageContentPart
{
    fn from(block: anthropic::ContentBlockSourceContent) -> Self {
        match block {
            anthropic::ContentBlockSourceContent::Text(block) => block.into(),
            anthropic::ContentBlockSourceContent::Image(block) => block.into(),
        }
    }
}

impl From<anthropic::ImageBlockParam> for chat::ChatCompletionRequestUserMessageContentPart {
    fn from(block: anthropic::ImageBlockParam) -> Self {
        match block.source {
            anthropic::ImageBlockSource::Url(source) => {
                Self::ImageUrl(chat::ChatCompletionRequestMessageContentPartImage {
                    image_url: chat::ImageUrl {
                        url: source.url,
                        detail: None,
                    },
                })
            }
            anthropic::ImageBlockSource::Base64(source) => {
                Self::ImageUrl(chat::ChatCompletionRequestMessageContentPartImage {
                    image_url: chat::ImageUrl {
                        url: format!("data:{};base64,{}", source.media_type.as_ref(), source.data),
                        detail: None,
                    },
                })
            }
        }
    }
}

// Tool results --------------------------------------------------------------

impl TryFrom<anthropic::ToolResultBlockParam> for chat::ChatCompletionRequestMessage {
    type Error = TranslationError;

    fn try_from(block: anthropic::ToolResultBlockParam) -> TranslationResult<Self> {
        Ok(Self::Tool(chat::ChatCompletionRequestToolMessage {
            content: block.content.try_into()?,
            tool_call_id: block.tool_use_id,
        }))
    }
}

impl From<&anthropic::TextBlockParam> for chat::ChatCompletionRequestToolMessageContentPart {
    fn from(block: &anthropic::TextBlockParam) -> Self {
        Self::Text(block.text.clone().into())
    }
}

impl TryFrom<&anthropic::ToolResultContentBlockParam>
    for chat::ChatCompletionRequestToolMessageContentPart
{
    type Error = TranslationError;

    fn try_from(block: &anthropic::ToolResultContentBlockParam) -> TranslationResult<Self> {
        match block {
            anthropic::ToolResultContentBlockParam::Text(block) => Ok(block.into()),
            other => Err(TranslationError::InvalidPayload(format!(
                "Anthropic tool_result content block `{}` cannot be translated to Chat Completions tool message content; Chat tool messages only support text content",
                other.as_ref()
            ))),
        }
    }
}

impl TryFrom<Option<anthropic::ToolResultContentParam>>
    for chat::ChatCompletionRequestToolMessageContent
{
    type Error = TranslationError;

    fn try_from(content: Option<anthropic::ToolResultContentParam>) -> TranslationResult<Self> {
        match content {
            Some(anthropic::ToolResultContentParam::Text(text)) => Ok(Self::Text(text)),
            Some(anthropic::ToolResultContentParam::Blocks(blocks)) => {
                let parts = blocks
                    .iter()
                    .map(TryInto::try_into)
                    .collect::<TranslationResult<Vec<_>>>()?;

                Ok(Self::Array(parts))
            }
            // Chat tool message content is required; an omitted Anthropic tool result
            // is represented as an empty text result rather than inventing content.
            None => Ok(Self::Text(String::new())),
        }
    }
}
