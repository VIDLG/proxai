use async_openai::types::chat as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::PromptTokensDetails))]
pub struct PromptTokensDetails {
    pub audio_tokens: Option<u32>,
    pub cached_tokens: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CompletionTokensDetails))]
pub struct CompletionTokensDetails {
    pub accepted_prediction_tokens: Option<u32>,
    pub audio_tokens: Option<u32>,
    pub reasoning_tokens: Option<u32>,
    pub rejected_prediction_tokens: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CompletionUsage))]
pub struct CompletionUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    pub completion_tokens_details: Option<CompletionTokensDetails>,
}
