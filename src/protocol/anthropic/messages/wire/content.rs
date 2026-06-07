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
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text(TextBlock),
    #[serde(rename = "thinking")]
    Thinking(ThinkingBlock),
    #[serde(rename = "redacted_thinking")]
    RedactedThinking(RedactedThinkingBlock),
    #[serde(rename = "tool_use")]
    ToolUse(ToolUseBlock),
    #[serde(rename = "server_tool_use")]
    ServerToolUse(ServerToolUseBlock),
    #[serde(rename = "web_search_tool_result")]
    WebSearchToolResult(WebSearchToolResultBlock),
    #[serde(rename = "web_fetch_tool_result")]
    WebFetchToolResult(WebFetchToolResultBlock),
    #[serde(rename = "code_execution_tool_result")]
    CodeExecutionToolResult(CodeExecutionToolResultBlock),
    #[serde(rename = "bash_code_execution_tool_result")]
    BashCodeExecutionToolResult(BashCodeExecutionToolResultBlock),
    #[serde(rename = "text_editor_code_execution_tool_result")]
    TextEditorCodeExecutionToolResult(TextEditorCodeExecutionToolResultBlock),
    #[serde(rename = "tool_search_tool_result")]
    ToolSearchToolResult(ToolSearchToolResultBlock),
    #[serde(rename = "container_upload")]
    ContainerUpload(ContainerUploadBlock),
}

/// 🎯 @use: request-side content block union.
/// Used by: message
#[derive(Debug, Clone, PartialEq, Eq, AsRefStr, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(tag = "type")]
pub enum ContentBlockParam {
    #[serde(rename = "text")]
    Text(TextBlockParam),
    #[serde(rename = "image")]
    Image(ImageBlockParam),
    #[serde(rename = "document")]
    Document(DocumentBlockParam),
    #[serde(rename = "search_result")]
    SearchResult(SearchResultBlockParam),
    #[serde(rename = "thinking")]
    Thinking(ThinkingBlockParam),
    #[serde(rename = "redacted_thinking")]
    RedactedThinking(RedactedThinkingBlockParam),
    #[serde(rename = "tool_use")]
    ToolUse(ToolUseBlockParam),
    #[serde(rename = "tool_result")]
    ToolResult(ToolResultBlockParam),
    #[serde(rename = "tool_reference")]
    ToolReference(ToolReferenceBlockParam),
    #[serde(rename = "server_tool_use")]
    ServerToolUse(ServerToolUseBlockParam),
    #[serde(rename = "web_search_tool_result")]
    WebSearchToolResult(WebSearchToolResultBlockParam),
    #[serde(rename = "web_fetch_tool_result")]
    WebFetchToolResult(WebFetchToolResultBlockParam),
    #[serde(rename = "code_execution_tool_result")]
    CodeExecutionToolResult(CodeExecutionToolResultBlockParam),
    #[serde(rename = "bash_code_execution_tool_result")]
    BashCodeExecutionToolResult(BashCodeExecutionToolResultBlockParam),
    #[serde(rename = "text_editor_code_execution_tool_result")]
    TextEditorCodeExecutionToolResult(TextEditorCodeExecutionToolResultBlockParam),
    #[serde(rename = "tool_search_tool_result")]
    ToolSearchToolResult(ToolSearchToolResultBlockParam),
    #[serde(rename = "container_upload")]
    ContainerUpload(ContainerUploadBlockParam),
    #[serde(rename = "mid_conv_system")]
    MidConversationSystem(MidConversationSystemBlockParam),
}
