use async_openai::types::responses as openai;
use structural_convert::StructuralConvert;
use strum::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::ServiceTier))]
#[strum(serialize_all = "snake_case")]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Scale,
    Priority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, StructuralConvert, Display)]
#[convert(from(openai::Verbosity))]
#[strum(serialize_all = "lowercase")]
pub enum Verbosity {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::OutputStatus))]
#[strum(serialize_all = "snake_case")]
pub enum OutputStatus {
    InProgress,
    Completed,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::Truncation))]
#[strum(serialize_all = "lowercase")]
pub enum Truncation {
    Auto,
    Disabled,
}
