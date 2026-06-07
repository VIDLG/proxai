use serde::{Deserialize, Serialize};
use strum::Display;

use super::super::{Annotation, OutputStatus, ReasoningTextContent};
use super::MessagePhase;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopLogProb {
    pub bytes: Vec<u8>,
    pub logprob: f64,
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogProb {
    pub bytes: Vec<u8>,
    pub logprob: f64,
    pub token: String,
    pub top_logprobs: Vec<TopLogProb>,
}

#[allow(
    dead_code,
    reason = "Retained for future response stream event logprob modeling."
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseTopLobProb {
    pub logprob: f64,
    pub token: String,
}

#[allow(
    dead_code,
    reason = "Retained for future response stream event logprob modeling."
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseLogProb {
    pub logprob: f64,
    pub token: String,
    pub top_logprobs: Vec<ResponseTopLobProb>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputTextContent {
    pub annotations: Vec<Annotation>,
    pub logprobs: Option<Vec<LogProb>>,
    pub text: String,
}

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputMessage {
    pub content: Vec<OutputMessageContent>,
    pub id: String,
    pub role: AssistantRole,
    pub phase: Option<MessagePhase>,
    pub status: OutputStatus,
}
