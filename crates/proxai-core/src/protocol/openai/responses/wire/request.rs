use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::{
    ContextManagementParam, Conversation, IncludeEnum, InputParam, Prompt, PromptCacheRetention,
    Reasoning, ResponseStreamOptions, ResponseTextParam, ServiceTier, Tool, ToolChoiceParam,
    Truncation,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConversationParam {
    ConversationID(String),
    Object(Conversation),
}

/// OpenAI Responses request wire envelope.
///
/// This is the protocol-native request shape accepted by `/v1/responses`.
/// Translation code should parse inbound Responses payloads through this type
/// instead of reading top-level request fields directly from `serde_json::Value`.
/// OpenAPI schema: `#/paths/~1responses/post/requestBody/content/application~1json/schema`
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CreateResponseRequest {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub metadata: OptionalNullable<HashMap<String, String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub top_logprobs: Option<u8>,
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
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub previous_response_id: OptionalNullable<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub background: OptionalNullable<bool>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub max_tool_calls: OptionalNullable<u32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub text: Option<ResponseTextParam>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub tools: Option<Vec<Tool>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub tool_choice: Option<ToolChoiceParam>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub prompt: OptionalNullable<Prompt>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub truncation: OptionalNullable<Truncation>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub reasoning: OptionalNullable<Reasoning>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub input: Option<InputParam>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub include: OptionalNullable<Vec<IncludeEnum>>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub parallel_tool_calls: OptionalNullable<bool>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub store: OptionalNullable<bool>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub instructions: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub moderation: OptionalNullable<Value>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub stream: OptionalNullable<bool>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub stream_options: OptionalNullable<ResponseStreamOptions>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub conversation: OptionalNullable<ConversationParam>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub context_management: OptionalNullable<Vec<ContextManagementParam>>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub max_output_tokens: OptionalNullable<u32>,
}
