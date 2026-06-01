use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;
use strum::Display;

use super::OutputStatus;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, StructuralConvert, Display, Serialize, Deserialize,
)]
#[convert(from(openai::ReasoningEffort))]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::ReasoningSummary))]
#[strum(serialize_all = "snake_case")]
pub enum ReasoningSummary {
    Auto,
    Concise,
    Detailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::Reasoning))]
pub struct Reasoning {
    pub effort: Option<ReasoningEffort>,
    pub summary: Option<ReasoningSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::SummaryTextContent))]
pub struct SummaryTextContent {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::SummaryPart))]
pub enum SummaryPart {
    SummaryText(SummaryTextContent),
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ReasoningTextContent))]
pub struct ReasoningTextContent {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ReasoningItemContent))]
pub enum ReasoningItemContent {
    ReasoningText(ReasoningTextContent),
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ReasoningItem))]
pub struct ReasoningItem {
    pub id: Option<String>,
    pub summary: Vec<SummaryPart>,
    pub content: Option<Vec<ReasoningItemContent>>,
    pub encrypted_content: Option<String>,
    pub status: Option<OutputStatus>,
}
