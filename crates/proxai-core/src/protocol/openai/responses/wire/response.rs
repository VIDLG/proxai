use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use strum::{Display, EnumString};

use crate::protocol::RequiredNullable;

use super::{
    InputItem, OutputItem, Prompt, PromptCacheRetention, Reasoning, ServiceTier, Tool,
    ToolChoiceParam, Truncation, Verbosity,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Instructions {
    Text(String),
    Array(Vec<InputItem>),
}

/// OpenAPI schema: `#/components/schemas/ResponseErrorCode`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum ResponseErrorCode {
    ServerError,
    RateLimitExceeded,
    InvalidPrompt,
    BioPolicy,
    VectorStoreTimeout,
    InvalidImage,
    InvalidImageFormat,
    InvalidBase64Image,
    InvalidImageUrl,
    ImageTooLarge,
    ImageTooSmall,
    ImageParseError,
    ImageContentPolicyViolation,
    InvalidImageMode,
    ImageFileTooLarge,
    UnsupportedImageMediaType,
    EmptyImageFile,
    FailedToDownloadImage,
    ImageFileNotFound,
}

/// OpenAPI schema: `#/components/schemas/ResponseError`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: ResponseErrorCode,
    pub message: String,
}

/// OpenAPI schema:
/// `#/components/schemas/Response/allOf/2/properties/incomplete_details/anyOf/0/properties/reason`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum IncompleteDetailsReason {
    ContentFilter,
    MaxOutputTokens,
}

/// OpenAPI schema: `#/components/schemas/Response/allOf/2/properties/incomplete_details/anyOf/0`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncompleteDetails {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub reason: Option<IncompleteDetailsReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Status {
    Completed,
    Failed,
    InProgress,
    Cancelled,
    #[default]
    Queued,
    Incomplete,
}

/// OpenAPI schema: `#/components/schemas/ResponseUsage/properties/input_tokens_details`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputTokenDetails {
    pub cached_tokens: u32,
    pub cache_write_tokens: u32,
}

/// OpenAPI schema: `#/components/schemas/ResponseUsage/properties/output_tokens_details`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputTokenDetails {
    pub reasoning_tokens: u32,
}

/// OpenAPI schema: `#/components/schemas/ResponseUsage`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseUsage {
    pub input_tokens: u32,
    pub input_tokens_details: InputTokenDetails,
    pub output_tokens: u32,
    pub output_tokens_details: OutputTokenDetails,
    pub total_tokens: u32,
}

// ── Conversation ─────────────────────────────────────────────

/// OpenAPI schema: `#/components/schemas/Conversation-2`
/// Rust name differs because `Conversation-2` is not a valid Rust identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
}

// ── Response formatting ─────────────────────────────────────

/// OpenAPI schema: `#/components/schemas/TextResponseFormatJsonSchema`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TextResponseFormatJsonSchema {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub description: Option<String>,
    pub name: String,
    pub schema: Value,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub strict: OptionalNullable<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Display, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TextResponseFormatConfiguration {
    #[default]
    Text,
    JsonObject,
    #[strum(to_string = "json_schema")]
    JsonSchema(TextResponseFormatJsonSchema),
}

/// OpenAPI schema: `#/components/schemas/ResponseTextParam`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ResponseTextParam {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub format: Option<TextResponseFormatConfiguration>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub verbosity: OptionalNullable<Verbosity>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ResponseObject {
    #[default]
    Response,
}

/// OpenAPI schema: `#/components/schemas/Response`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub metadata: RequiredNullable<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub top_logprobs: OptionalNullable<u8>,
    pub temperature: RequiredNullable<f32>,
    pub top_p: RequiredNullable<f32>,
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
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub previous_response_id: OptionalNullable<String>,
    pub model: String,
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
    pub tools: Vec<Tool>,
    pub tool_choice: ToolChoiceParam,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub prompt: OptionalNullable<Prompt>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub truncation: OptionalNullable<Truncation>,
    pub id: String,
    pub object: ResponseObject,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub status: Option<Status>,
    pub created_at: f64,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub completed_at: OptionalNullable<f64>,
    pub error: RequiredNullable<ResponseError>,
    pub incomplete_details: RequiredNullable<IncompleteDetails>,
    pub output: Vec<OutputItem>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub reasoning: OptionalNullable<Reasoning>,
    pub instructions: RequiredNullable<Instructions>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub output_text: OptionalNullable<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub usage: Option<ResponseUsage>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub prompt_cache_options: Option<Value>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub moderation: OptionalNullable<Value>,
    pub parallel_tool_calls: bool,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub conversation: OptionalNullable<Conversation>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub max_output_tokens: OptionalNullable<u32>,
}
