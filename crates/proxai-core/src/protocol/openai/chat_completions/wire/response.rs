use crate::protocol::RequiredNullable;
use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use strum::Display;

use super::{CompletionUsage, ServiceTier};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    FunctionCall,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum AssistantRole {
    #[default]
    Assistant,
}

/// OpenAPI schema:
/// `#/components/schemas/ChatCompletionMessageToolCall/properties/function`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionMessageToolCall`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionMessageToolCall {
    pub id: String,
    pub function: FunctionCall,
}

/// OpenAPI schema:
/// `#/components/schemas/ChatCompletionMessageCustomToolCall/properties/custom`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomTool {
    pub name: String,
    pub input: String,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionMessageCustomToolCall`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionMessageCustomToolCall {
    pub id: String,
    pub custom: CustomTool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionMessageToolCalls {
    Function(ChatCompletionMessageToolCall),
    Custom(ChatCompletionMessageCustomToolCall),
}

/// OpenAPI schema:
/// `#/components/schemas/ChatCompletionResponseMessage/properties/audio/anyOf/0`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionResponseMessageAudio {
    pub id: String,
    pub expires_at: u64,
    pub data: String,
    pub transcript: String,
}

/// OpenAPI schema:
/// `#/components/schemas/ChatCompletionResponseMessage/properties/annotations/items/properties/url_citation`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlCitation {
    pub end_index: u32,
    pub start_index: u32,
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionResponseMessageAnnotation {
    UrlCitation { url_citation: UrlCitation },
}

/// OpenAPI schema:
/// `#/components/schemas/ChatCompletionTokenLogprob/properties/top_logprobs/items`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopLogprobs {
    pub token: String,
    pub logprob: f32,
    pub bytes: RequiredNullable<Vec<u8>>,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionTokenLogprob`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionTokenLogprob {
    pub token: String,
    pub logprob: f32,
    pub bytes: RequiredNullable<Vec<u8>>,
    pub top_logprobs: Vec<TopLogprobs>,
}

/// OpenAPI schema:
/// `#/components/schemas/CreateChatCompletionResponse/properties/choices/items/properties/logprobs/anyOf/0`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatChoiceLogprobs {
    pub content: RequiredNullable<Vec<ChatCompletionTokenLogprob>>,
    pub refusal: RequiredNullable<Vec<ChatCompletionTokenLogprob>>,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionResponseMessage`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionResponseMessage {
    pub content: RequiredNullable<String>,
    pub refusal: RequiredNullable<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCalls>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub annotations: Option<Vec<ChatCompletionResponseMessageAnnotation>>,
    pub role: AssistantRole,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub function_call: Option<FunctionCall>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub audio: OptionalNullable<ChatCompletionResponseMessageAudio>,
}

/// OpenAPI schema: `#/components/schemas/CreateChatCompletionResponse/properties/choices/items`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatChoice {
    pub finish_reason: FinishReason,

    pub index: u32,
    pub message: ChatCompletionResponseMessage,
    pub logprobs: RequiredNullable<ChatChoiceLogprobs>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum CreateChatCompletionResponseObject {
    #[default]
    #[serde(rename = "chat.completion")]
    #[strum(to_string = "chat.completion")]
    ChatCompletion,
}

/// OpenAPI schema: `#/components/schemas/CreateChatCompletionResponse`
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CreateChatCompletionResponse {
    pub id: String,
    pub choices: Vec<ChatChoice>,
    pub created: u32,
    pub model: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub service_tier: OptionalNullable<ServiceTier>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub system_fingerprint: Option<String>,
    pub object: CreateChatCompletionResponseObject,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub usage: Option<CompletionUsage>,
}
