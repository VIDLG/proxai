#![allow(
    deprecated,
    reason = "Chat Completions wire compatibility includes deprecated max_tokens."
)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::Display;

use super::super::request::wire::{
    ChatCompletionAudio, ChatCompletionRequestMessageContentPartText, ChatCompletionStreamOptions,
    ChatCompletionToolChoiceOption, ChatCompletionTools, PredictionContent, ReasoningEffort,
    ResponseFormat, ResponseModalities, StopConfiguration, Verbosity, WebSearchOptions,
};
use super::{
    ChatCompletionMessageToolCalls, ChatCompletionResponseMessage, CreateChatCompletionResponse,
    ServiceTier,
};

// ============================================================
// ImageUrl
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ImageDetail {
    #[default]
    Auto,
    Low,
    High,
    Original,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    pub detail: Option<ImageDetail>,
}

// ============================================================
// Input Audio
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum InputAudioFormat {
    Wav,
    #[default]
    Mp3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputAudio {
    pub data: String,
    pub format: InputAudioFormat,
}

// ============================================================
// File
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileObject {
    pub file_data: Option<String>,
    pub file_id: Option<String>,
    pub filename: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageContentPartFile {
    pub file: FileObject,
}

// ============================================================
// Content Parts
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageContentPartRefusal {
    pub refusal: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageContentPartImage {
    pub image_url: ImageUrl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageContentPartAudio {
    pub input_audio: InputAudio,
}

// ============================================================
// Message Content Enums
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionRequestSystemMessageContentPart {
    Text(ChatCompletionRequestMessageContentPartText),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionRequestUserMessageContentPart {
    Text(ChatCompletionRequestMessageContentPartText),
    ImageUrl(ChatCompletionRequestMessageContentPartImage),
    InputAudio(ChatCompletionRequestMessageContentPartAudio),
    File(ChatCompletionRequestMessageContentPartFile),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionRequestAssistantMessageContentPart {
    Text(ChatCompletionRequestMessageContentPartText),
    Refusal(ChatCompletionRequestMessageContentPartRefusal),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionRequestToolMessageContentPart {
    Text(ChatCompletionRequestMessageContentPartText),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionRequestDeveloperMessageContentPart {
    Text(ChatCompletionRequestMessageContentPartText),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionRequestSystemMessageContent {
    Text(String),
    Array(Vec<ChatCompletionRequestSystemMessageContentPart>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionRequestUserMessageContent {
    Text(String),
    Array(Vec<ChatCompletionRequestUserMessageContentPart>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionRequestAssistantMessageContent {
    Text(String),
    Array(Vec<ChatCompletionRequestAssistantMessageContentPart>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionRequestToolMessageContent {
    Text(String),
    Array(Vec<ChatCompletionRequestToolMessageContentPart>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionRequestDeveloperMessageContent {
    Text(String),
    Array(Vec<ChatCompletionRequestDeveloperMessageContentPart>),
}

// ============================================================
// Message Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestDeveloperMessage {
    pub content: ChatCompletionRequestDeveloperMessageContent,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestSystemMessage {
    pub content: ChatCompletionRequestSystemMessageContent,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestUserMessage {
    pub content: ChatCompletionRequestUserMessageContent,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestAssistantMessageAudio {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestAssistantMessage {
    pub content: Option<ChatCompletionRequestAssistantMessageContent>,
    pub refusal: Option<String>,
    pub name: Option<String>,
    pub audio: Option<ChatCompletionRequestAssistantMessageAudio>,
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCalls>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestToolMessage {
    pub content: ChatCompletionRequestToolMessageContent,
    pub tool_call_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestFunctionMessage {
    pub content: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum ChatCompletionRequestMessage {
    Developer(ChatCompletionRequestDeveloperMessage),
    System(ChatCompletionRequestSystemMessage),
    User(ChatCompletionRequestUserMessage),
    Assistant(ChatCompletionRequestAssistantMessage),
    Tool(ChatCompletionRequestToolMessage),
    Function(ChatCompletionRequestFunctionMessage),
}

// ============================================================
// Response-level types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CompletionFinishReason {
    Stop,
    Length,
    ContentFilter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Logprobs {
    pub tokens: Vec<String>,
    pub token_logprobs: Vec<Option<f32>>,
    pub top_logprobs: Vec<serde_json::Value>,
    pub text_offset: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub text: String,
    pub index: u32,
    pub logprobs: Option<Logprobs>,
    pub finish_reason: Option<CompletionFinishReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text(ChatCompletionRequestMessageContentPartText),
    ImageUrl(ChatCompletionRequestMessageContentPartImage),
}

// ============================================================
// Request/Response wrapper types
// ============================================================

#[allow(deprecated)]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CreateChatCompletionRequest {
    pub messages: Vec<ChatCompletionRequestMessage>,
    pub model: String,
    pub modalities: Option<Vec<ResponseModalities>>,
    pub verbosity: Option<Verbosity>,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub max_completion_tokens: Option<u32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub web_search_options: Option<WebSearchOptions>,
    pub top_logprobs: Option<u8>,
    pub response_format: Option<ResponseFormat>,
    pub audio: Option<ChatCompletionAudio>,
    pub store: Option<bool>,
    pub stream: Option<bool>,
    pub stop: Option<StopConfiguration>,
    pub logit_bias: Option<HashMap<String, i8>>,
    pub logprobs: Option<bool>,
    pub max_tokens: Option<u32>,
    pub n: Option<u8>,
    pub prediction: Option<PredictionContent>,
    pub stream_options: Option<ChatCompletionStreamOptions>,
    pub service_tier: Option<ServiceTier>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub tools: Option<Vec<ChatCompletionTools>>,
    pub tool_choice: Option<ChatCompletionToolChoiceOption>,
    pub parallel_tool_calls: Option<bool>,
    pub safety_identifier: Option<String>,
    pub prompt_cache_key: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionDeleted {
    pub object: String,
    pub id: String,
    pub deleted: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionList {
    pub object: String,
    pub data: Vec<CreateChatCompletionResponse>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionMessageListItem {
    pub id: String,
    pub content_parts: Option<Vec<ContentPart>>,
    pub message: ChatCompletionResponseMessage,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionMessageList {
    pub object: String,
    pub data: Vec<ChatCompletionMessageListItem>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateChatCompletionRequest {
    pub metadata: serde_json::Value,
}

// ============================================================
// Prompt
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Prompt {
    String(String),
    StringArray(Vec<String>),
    IntegerArray(Vec<u32>),
    ArrayOfIntegerArray(Vec<Vec<u32>>),
}

// ============================================================
// ChatCompletionResponseStream (type alias, not a struct/enum)
// ============================================================
// SDK: pub type ChatCompletionResponseStream = StreamResponse<CreateChatCompletionStreamResponse>;
// proxai handles SSE at the byte/event level, so this type alias is not needed.
