use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use strum::Display;

use super::super::{Annotation, OutputStatus, ReasoningTextContent};
use super::MessagePhase;

/// OpenAPI schema: `#/components/schemas/TopLogProb`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopLogProb {
    pub token: String,
    pub logprob: f64,

    pub bytes: Vec<u8>,
}

/// OpenAPI schema: `#/components/schemas/LogProb`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogProb {
    pub token: String,
    pub logprob: f64,

    pub bytes: Vec<u8>,
    pub top_logprobs: Vec<TopLogProb>,
}

/// OpenAPI schema: `#/components/schemas/ResponseLogProb/properties/top_logprobs/items`
#[allow(
    dead_code,
    reason = "Retained for future response stream event logprob modeling."
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseTopLogProb {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub token: Option<String>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub logprob: Option<f64>,
}

/// OpenAPI schema: `#/components/schemas/ResponseLogProb`
#[allow(
    dead_code,
    reason = "Retained for future response stream event logprob modeling."
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseLogProb {
    pub token: String,

    pub logprob: f64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub top_logprobs: Option<Vec<ResponseTopLogProb>>,
}

/// OpenAPI schema: `#/components/schemas/OutputTextContent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputTextContent {
    pub text: String,

    pub annotations: Vec<Annotation>,
    pub logprobs: Vec<LogProb>,
}

/// OpenAPI schema: `#/components/schemas/RefusalContent`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefusalContent {
    pub refusal: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputMessageContent {
    OutputText(OutputTextContent),
    Refusal(RefusalContent),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputContent {
    OutputText(OutputTextContent),
    Refusal(RefusalContent),
    ReasoningText(ReasoningTextContent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum AssistantRole {
    #[default]
    Assistant,
}

/// OpenAPI schema: `#/components/schemas/OutputMessage`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputMessage {
    pub id: String,
    pub role: AssistantRole,

    pub content: Vec<OutputMessageContent>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub phase: OptionalNullable<MessagePhase>,
    pub status: OutputStatus,
}
