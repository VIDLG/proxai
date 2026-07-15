use crate::protocol::RequiredNullable;
use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};

use strum::Display;

use super::{ChatChoiceLogprobs, CompletionUsage, FinishReason, ServiceTier};

/// OpenAPI schema:
/// `#/components/schemas/ChatCompletionStreamResponseDelta/properties/function_call`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCallStream {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub arguments: Option<String>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionType {
    Function,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionMessageToolCallChunk`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionMessageToolCallChunk {
    pub index: u32,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub r#type: Option<FunctionType>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub function: Option<FunctionCallStream>,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionStreamResponseDelta/properties/role`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ChatCompletionStreamRole {
    Assistant,
    Developer,
    System,
    Tool,
    User,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionStreamResponseDelta`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionStreamResponseDelta {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub content: OptionalNullable<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub function_call: Option<FunctionCallStream>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCallChunk>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub role: Option<ChatCompletionStreamRole>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub refusal: OptionalNullable<String>,
}

/// OpenAPI schema:
/// `#/components/schemas/CreateChatCompletionStreamResponse/properties/choices/items`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatChoiceStream {
    pub delta: ChatCompletionStreamResponseDelta,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub logprobs: OptionalNullable<ChatChoiceLogprobs>,
    pub finish_reason: RequiredNullable<FinishReason>,

    pub index: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum CreateChatCompletionStreamResponseObject {
    #[default]
    #[serde(rename = "chat.completion.chunk")]
    #[strum(to_string = "chat.completion.chunk")]
    ChatCompletionChunk,
}

/// OpenAPI schema: `#/components/schemas/CreateChatCompletionStreamResponse`
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CreateChatCompletionStreamResponse {
    pub id: String,
    pub choices: Vec<ChatChoiceStream>,
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
    pub object: CreateChatCompletionStreamResponseObject,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub usage: OptionalNullable<CompletionUsage>,
}
