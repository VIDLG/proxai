use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use strum::Display;

use super::OutputStatus;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReasoningSummary {
    Auto,
    Concise,
    Detailed,
}

/// OpenAPI schema: `#/components/schemas/Reasoning`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Reasoning {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub effort: OptionalNullable<ReasoningEffort>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub summary: OptionalNullable<ReasoningSummary>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub generate_summary: OptionalNullable<ReasoningSummary>,
}

/// OpenAPI schema: `#/components/schemas/SummaryTextContent`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryTextContent {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SummaryPart {
    SummaryText(SummaryTextContent),
}

/// OpenAPI schema: `#/components/schemas/ReasoningTextContent`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReasoningTextContent {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReasoningItemContent {
    ReasoningText(ReasoningTextContent),
}

/// OpenAPI schema: `#/components/schemas/ReasoningItem`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReasoningItem {
    pub id: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub encrypted_content: OptionalNullable<String>,
    pub summary: Vec<SummaryPart>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub content: Option<Vec<ReasoningItemContent>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub status: Option<OutputStatus>,
}
