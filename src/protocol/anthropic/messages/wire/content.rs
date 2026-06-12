#![allow(
    dead_code,
    reason = "Anthropic Messages content block aggregation enums and content-only types."
)]

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextBlock {
    /// @sdk(required_nullable_accepts_missing)
    pub citations: Option<Vec<TextCitation>>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingBlock {
    pub signature: String,
    pub thinking: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedThinkingBlock {
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerUploadBlock {
    pub file_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerUploadBlockParam {
    pub file_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MidConversationSystemBlockParam {
    pub content: Vec<TextBlockParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlEphemeral>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingBlockParam {
    pub signature: String,
    pub thinking: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedThinkingBlockParam {
    pub data: String,
}

/// 🎯 @use: response-side content block union.
/// Used by: message, stream
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    MidConversationSystem(MidConversationSystemBlockParam),
}
