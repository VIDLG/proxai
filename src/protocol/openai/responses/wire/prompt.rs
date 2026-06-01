use async_openai::types::responses as openai;
use serde_json::Value;
use structural_convert::StructuralConvert;
use strum::Display;

use super::InputContent;

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ResponsePromptVariables))]
pub enum ResponsePromptVariables {
    String(String),
    Content(InputContent),
    Custom(Value),
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::Prompt))]
pub struct Prompt {
    pub id: String,
    pub version: Option<String>,
    pub variables: Option<ResponsePromptVariables>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::PromptCacheRetention))]
#[strum(serialize_all = "snake_case")]
pub enum PromptCacheRetention {
    InMemory,
    #[strum(to_string = "24h")]
    Hours24,
}
