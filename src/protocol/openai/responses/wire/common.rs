use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;
use strum::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::ServiceTier))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Scale,
    Priority,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, StructuralConvert, Display, Serialize, Deserialize,
)]
#[convert(from(openai::Verbosity))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Verbosity {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::OutputStatus))]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OutputStatus {
    InProgress,
    Completed,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::Truncation))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Truncation {
    Auto,
    Disabled,
}
