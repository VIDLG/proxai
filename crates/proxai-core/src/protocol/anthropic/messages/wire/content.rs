#![allow(
    dead_code,
    reason = "Anthropic Messages content block aggregation enums and content-only types."
)]

use crate::protocol::OptionalNullable;
use crate::protocol::RequiredNullable;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;

use super::{
    blocks::{DocumentBlockParam, ImageBlockParam, TextBlockParam},
    citations::TextCitation,
    common::CacheControlEphemeral,
    tools::{
        ServerToolUseBlock, ServerToolUseBlockParam, ToolResultBlockParam, ToolUseBlock,
        ToolUseBlockParam,
        bash::{BashCodeExecutionToolResultBlock, BashCodeExecutionToolResultBlockParam},
        code_execution::{CodeExecutionToolResultBlock, CodeExecutionToolResultBlockParam},
        search::{
            SearchResultBlockParam, ToolSearchToolResultBlock, ToolSearchToolResultBlockParam,
        },
        text_editor::{
            TextEditorCodeExecutionToolResultBlock, TextEditorCodeExecutionToolResultBlockParam,
        },
        tool_use::ToolReferenceBlockParam,
        web::{
            WebFetchToolResultBlock, WebFetchToolResultBlockParam, WebSearchToolResultBlock,
            WebSearchToolResultBlockParam,
        },
    },
};

/// Discriminator value used by `ThinkingBlock.type`.
/// @sdk(proxai_internal = "discriminator")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingContentType {
    Thinking,
}

/// Discriminator value used by `RedactedThinkingBlock.type`.
/// @sdk(proxai_internal = "discriminator")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactedThinkingBlockType {
    RedactedThinking,
}

/// Discriminator value used by `ContainerUploadBlock.type`.
/// @sdk(proxai_internal = "discriminator")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerUploadType {
    ContainerUpload,
}

/// @sdk(shape = "TextBlock")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextBlock {
    pub citations: RequiredNullable<Vec<TextCitation>>,
    pub text: String,
}

/// @sdk(shape = "ThinkingBlock")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingBlock {
    pub signature: String,
    pub thinking: String,
}

/// @sdk(shape = "RedactedThinkingBlock")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedThinkingBlock {
    pub data: String,
}

/// @sdk(shape = "ContainerUploadBlock")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerUploadBlock {
    pub file_id: String,
}

/// @sdk(shape = "ContainerUploadBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerUploadBlockParam {
    pub file_id: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
}

/// @sdk(shape = "MidConversationSystemBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MidConversationSystemBlockParam {
    pub content: Vec<TextBlockParam>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub cache_control: OptionalNullable<CacheControlEphemeral>,
}

/// @sdk(shape = "ThinkingBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingBlockParam {
    pub signature: String,
    pub thinking: String,
}

/// @sdk(shape = "RedactedThinkingBlockParam")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedThinkingBlockParam {
    pub data: String,
}

/// @sdk(shape = "ContentBlock")
/// 🎯 @use: response-side content block union.
/// Used by: message, stream
#[derive(Debug, Clone, PartialEq, Eq, AsRefStr, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text(TextBlock),
    Thinking(ThinkingBlock),
    RedactedThinking(RedactedThinkingBlock),
    ToolUse(ToolUseBlock),
    ServerToolUse(ServerToolUseBlock),
    WebSearchToolResult(WebSearchToolResultBlock),
    WebFetchToolResult(WebFetchToolResultBlock),
    CodeExecutionToolResult(CodeExecutionToolResultBlock),
    BashCodeExecutionToolResult(BashCodeExecutionToolResultBlock),
    TextEditorCodeExecutionToolResult(TextEditorCodeExecutionToolResultBlock),
    ToolSearchToolResult(ToolSearchToolResultBlock),
    ContainerUpload(ContainerUploadBlock),
}

/// @sdk(shape = "ContentBlockParam")
/// 🎯 @use: request-side content block union.
/// Used by: message
#[derive(Debug, Clone, PartialEq, Eq, AsRefStr, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockParam {
    Text(TextBlockParam),
    Image(ImageBlockParam),
    Document(DocumentBlockParam),
    SearchResult(SearchResultBlockParam),
    Thinking(ThinkingBlockParam),
    RedactedThinking(RedactedThinkingBlockParam),
    ToolUse(ToolUseBlockParam),
    ToolResult(ToolResultBlockParam),
    ToolReference(ToolReferenceBlockParam),
    ServerToolUse(ServerToolUseBlockParam),
    WebSearchToolResult(WebSearchToolResultBlockParam),
    WebFetchToolResult(WebFetchToolResultBlockParam),
    CodeExecutionToolResult(CodeExecutionToolResultBlockParam),
    BashCodeExecutionToolResult(BashCodeExecutionToolResultBlockParam),
    TextEditorCodeExecutionToolResult(TextEditorCodeExecutionToolResultBlockParam),
    ToolSearchToolResult(ToolSearchToolResultBlockParam),
    ContainerUpload(ContainerUploadBlockParam),
    #[strum(serialize = "mid_conv_system")]
    #[serde(rename = "mid_conv_system")]
    MidConversationSystem(MidConversationSystemBlockParam),
}
