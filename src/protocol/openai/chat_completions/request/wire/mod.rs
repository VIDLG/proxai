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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseFormatJsonSchema {
    pub description: Option<String>,
    pub name: String,
    pub schema: Option<Value>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionRequestMessageContentPartText {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PredictionContentContent {
    Text(String),
    Array(Vec<ChatCompletionRequestMessageContentPartText>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PredictionContent {
    Content(PredictionContentContent),
}

#[allow(
    dead_code,
    reason = "Retained for full request schema projection coverage."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionStreamOptions {
    pub include_usage: Option<bool>,
    pub include_obfuscation: Option<bool>,
}

// Deprecated upstream request fields not projected: `function_call`, `functions`.
