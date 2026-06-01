use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use structural_convert::StructuralConvert;
use strum::Display;

use crate::protocol::ErrorObject;

use super::{
    InputItem, OutputItem, Prompt, PromptCacheRetention, Reasoning, ServiceTier, Tool,
    ToolChoiceParam, Truncation, Verbosity,
};

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::Instructions))]
pub enum Instructions {
    Text(String),
    Array(Vec<InputItem>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, StructuralConvert, Deserialize)]
#[convert(from(openai::Billing))]
pub struct Billing {
    pub payer: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::IncompleteDetails))]
pub struct IncompleteDetails {
    pub reason: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default, Serialize, Deserialize,
)]
#[convert(from(openai::Status))]
#[strum(serialize_all = "snake_case")]
pub enum Status {
    Completed,
    Failed,
    InProgress,
    Cancelled,
    #[default]
    Queued,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::InputTokenDetails))]
pub struct InputTokenDetails {
    pub cached_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::OutputTokenDetails))]
pub struct OutputTokenDetails {
    pub reasoning_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseUsage))]
pub struct ResponseUsage {
    pub input_tokens: u32,
    pub input_tokens_details: InputTokenDetails,
    pub output_tokens: u32,
    pub output_tokens_details: OutputTokenDetails,
    pub total_tokens: u32,
}

// ── Conversation ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, StructuralConvert, Deserialize)]
#[convert(from(openai::Conversation))]
pub struct Conversation {
    pub id: String,
}

// ── Response formatting ─────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Default, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseFormatJsonSchema))]
pub struct ResponseFormatJsonSchema {
    pub description: Option<String>,
    pub name: String,
    pub schema: Option<Value>,
    pub strict: Option<bool>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Default, StructuralConvert, Display, Serialize, Deserialize,
)]
#[convert(from(openai::TextResponseFormatConfiguration))]
#[strum(serialize_all = "snake_case")]
pub enum TextResponseFormatConfiguration {
    #[default]
    Text,
    JsonObject,
    #[strum(to_string = "json_schema")]
    JsonSchema(ResponseFormatJsonSchema),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseTextParam))]
pub struct ResponseTextParam {
    pub format: TextResponseFormatConfiguration,
    pub verbosity: Option<Verbosity>,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::Response))]
pub struct Response {
    pub background: Option<bool>,
    pub billing: Option<Billing>,
    pub conversation: Option<Conversation>,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    pub error: Option<ErrorObject>,
    pub id: String,
    pub incomplete_details: Option<IncompleteDetails>,
    pub instructions: Option<Instructions>,
    pub max_output_tokens: Option<u32>,
    pub metadata: Option<HashMap<String, String>>,
    pub model: String,
    pub object: String,
    pub output: Vec<OutputItem>,
    pub parallel_tool_calls: Option<bool>,
    pub previous_response_id: Option<String>,
    pub prompt: Option<Prompt>,
    pub prompt_cache_key: Option<String>,
    pub prompt_cache_retention: Option<PromptCacheRetention>,
    pub reasoning: Option<Reasoning>,
    pub safety_identifier: Option<String>,
    pub service_tier: Option<ServiceTier>,
    pub status: Status,
    pub temperature: Option<f32>,
    pub text: Option<ResponseTextParam>,
    pub tool_choice: Option<ToolChoiceParam>,
    pub tools: Option<Vec<Tool>>,
    pub top_logprobs: Option<u8>,
    pub top_p: Option<f32>,
    pub truncation: Option<Truncation>,
    pub usage: Option<ResponseUsage>,
}
