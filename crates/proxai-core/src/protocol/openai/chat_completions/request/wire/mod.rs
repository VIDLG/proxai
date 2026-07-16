use crate::protocol::openai::PromptCacheBreakpointParam;
use serde::{Deserialize, Serialize};
mod audio;
mod tools;
mod web_search;

use serde_json::Value;
use strum::Display;

pub use self::audio::*;
pub use self::tools::*;
pub use self::web_search::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseModalities {
    Text,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StopConfiguration {
    String(String),
    StringArray(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    #[default]
    Medium,
    High,
    Xhigh,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Verbosity {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum PromptCacheRetention {
    #[default]
    #[serde(rename = "in_memory")]
    #[strum(to_string = "in_memory")]
    InMemory,
    #[serde(rename = "24h")]
    #[strum(to_string = "24h")]
    Hours24,
}

/// OpenAPI schema: `#/components/schemas/ResponseFormatJsonSchemaSchema`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseFormatJsonSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub name: String,
    pub schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    Text,
    JsonObject,
    JsonSchema {
        json_schema: ResponseFormatJsonSchema,
    },
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionRequestMessageContentPartText`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageContentPartText {
    pub text: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::protocol::deserialize_present"
    )]
    pub prompt_cache_breakpoint: Option<PromptCacheBreakpointParam>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PredictionContentContent {
    Text(String),
    Array(Vec<ChatCompletionRequestMessageContentPartText>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", content = "content")]
pub enum PredictionContent {
    Content(PredictionContentContent),
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionStreamOptions`
#[allow(
    dead_code,
    reason = "Retained for full request schema projection coverage."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionStreamOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_obfuscation: Option<bool>,
}

// Deprecated upstream request fields not projected: `function_call`, `functions`.
