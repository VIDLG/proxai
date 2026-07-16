#![allow(
    deprecated,
    reason = "Chat Completions wire compatibility includes deprecated max_tokens."
)]

use crate::protocol::RequiredNullable;
use crate::protocol::openai::PromptCacheBreakpointParam;
use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use strum::Display;

use super::super::request::wire::{
    ChatCompletionAudio, ChatCompletionFunctionCall, ChatCompletionFunctions,
    ChatCompletionRequestMessageContentPartText, ChatCompletionStreamOptions,
    ChatCompletionToolChoiceOption, ChatCompletionTools, PredictionContent, PromptCacheRetention,
    ReasoningEffort, ResponseFormat, ResponseModalities, StopConfiguration, Verbosity,
    WebSearchOptions,
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
}

/// OpenAPI schema:
/// `#/components/schemas/ChatCompletionRequestMessageContentPartImage/properties/image_url`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
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

/// OpenAPI schema:
/// `#/components/schemas/ChatCompletionRequestMessageContentPartAudio/properties/input_audio`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputAudioPayload {
    pub data: String,
    pub format: InputAudioFormat,
}

// ============================================================
// File
// ============================================================

/// OpenAPI schema:
/// `#/components/schemas/ChatCompletionRequestMessageContentPartFile/properties/file`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileObject {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub filename: Option<String>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub file_data: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub file_id: Option<String>,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionRequestMessageContentPartFile`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageContentPartFile {
    pub file: FileObject,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub prompt_cache_breakpoint: Option<PromptCacheBreakpointParam>,
}

// ============================================================
// Content Parts
// ============================================================

/// OpenAPI schema: `#/components/schemas/ChatCompletionRequestMessageContentPartRefusal`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageContentPartRefusal {
    pub refusal: String,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionRequestMessageContentPartImage`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageContentPartImage {
    pub image_url: ImageUrl,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub prompt_cache_breakpoint: Option<PromptCacheBreakpointParam>,
}

/// OpenAPI schema: `#/components/schemas/InputAudio`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageContentPartAudio {
    pub input_audio: InputAudioPayload,
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

/// OpenAPI schema: `#/components/schemas/ChatCompletionRequestDeveloperMessage`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestDeveloperMessage {
    pub content: ChatCompletionRequestDeveloperMessageContent,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub name: Option<String>,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionRequestSystemMessage`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestSystemMessage {
    pub content: ChatCompletionRequestSystemMessageContent,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub name: Option<String>,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionRequestUserMessage`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestUserMessage {
    pub content: ChatCompletionRequestUserMessageContent,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub name: Option<String>,
}

/// OpenAPI schema:
/// `#/components/schemas/ChatCompletionRequestAssistantMessage/properties/audio/anyOf/0`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestAssistantMessageAudio {
    pub id: String,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionRequestAssistantMessage`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestAssistantMessage {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub content: OptionalNullable<ChatCompletionRequestAssistantMessageContent>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub refusal: OptionalNullable<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub audio: OptionalNullable<ChatCompletionRequestAssistantMessageAudio>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCalls>>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub function_call: OptionalNullable<super::FunctionCall>,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionRequestToolMessage`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestToolMessage {
    pub content: ChatCompletionRequestToolMessageContent,
    pub tool_call_id: String,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionRequestFunctionMessage`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestFunctionMessage {
    pub content: RequiredNullable<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text(ChatCompletionRequestMessageContentPartText),
    ImageUrl(ChatCompletionRequestMessageContentPartImage),
}

// ============================================================
// Request/Response wrapper types
// ============================================================

/// OpenAPI schema: `#/components/schemas/CreateChatCompletionRequest`
#[allow(deprecated)]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CreateChatCompletionRequest {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub metadata: OptionalNullable<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub top_logprobs: OptionalNullable<u8>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub temperature: OptionalNullable<f32>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub top_p: OptionalNullable<f32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub user: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub safety_identifier: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub prompt_cache_key: Option<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub service_tier: OptionalNullable<ServiceTier>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub prompt_cache_retention: OptionalNullable<PromptCacheRetention>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub prompt_cache_options: Option<Value>,

    pub messages: Vec<ChatCompletionRequestMessage>,
    pub model: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub modalities: OptionalNullable<Vec<ResponseModalities>>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub verbosity: OptionalNullable<Verbosity>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub reasoning_effort: OptionalNullable<ReasoningEffort>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub max_completion_tokens: OptionalNullable<u32>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub frequency_penalty: OptionalNullable<f32>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub presence_penalty: OptionalNullable<f32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub web_search_options: Option<WebSearchOptions>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub response_format: Option<ResponseFormat>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub audio: OptionalNullable<ChatCompletionAudio>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub store: OptionalNullable<bool>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub moderation: OptionalNullable<Value>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub stream: OptionalNullable<bool>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub stop: OptionalNullable<StopConfiguration>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub logit_bias: OptionalNullable<HashMap<String, i8>>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub logprobs: OptionalNullable<bool>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub max_tokens: OptionalNullable<u32>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub n: OptionalNullable<u8>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub prediction: OptionalNullable<PredictionContent>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub seed: OptionalNullable<i64>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub stream_options: OptionalNullable<ChatCompletionStreamOptions>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub tools: Option<Vec<ChatCompletionTools>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub tool_choice: Option<ChatCompletionToolChoiceOption>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub parallel_tool_calls: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub function_call: Option<ChatCompletionFunctionCall>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub functions: Option<Vec<ChatCompletionFunctions>>,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionDeleted/properties/object`
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum ChatCompletionDeletedObject {
    #[default]
    #[serde(rename = "chat.completion.deleted")]
    #[strum(to_string = "chat.completion.deleted")]
    ChatCompletionDeleted,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionList/properties/object`
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum ChatCompletionListObject {
    #[default]
    #[serde(rename = "list")]
    #[strum(to_string = "list")]
    List,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionDeleted`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionDeleted {
    pub object: ChatCompletionDeletedObject,
    pub id: String,
    pub deleted: bool,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionList`
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionList {
    pub object: ChatCompletionListObject,
    pub data: Vec<CreateChatCompletionResponse>,
    pub first_id: String,
    pub last_id: String,
    pub has_more: bool,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionMessageList/properties/data/items`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionMessageListItem {
    /// The official item schema composes `ChatCompletionResponseMessage` via
    /// `allOf`; flattening preserves that wire shape without duplicating fields.
    #[serde(flatten)]
    pub message: ChatCompletionResponseMessage,
    pub id: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub content_parts: OptionalNullable<Vec<ContentPart>>,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionMessageList`
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionMessageList {
    pub object: ChatCompletionListObject,
    pub data: Vec<ChatCompletionMessageListItem>,
    pub first_id: String,
    pub last_id: String,
    pub has_more: bool,
}

/// OpenAPI schema:
/// `#/paths/~1chat~1completions~1{completion_id}/post/requestBody/content/application~1json/schema`
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateChatCompletionRequest {
    pub metadata: RequiredNullable<serde_json::Value>,
}

// ============================================================
// Prompt
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
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
