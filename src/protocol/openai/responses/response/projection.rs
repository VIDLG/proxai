//! OpenAI Responses protocol-native response projection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::protocol::ErrorObject;

use super::super::wire::ServiceTier;

use super::super::wire::{
    Billing, Conversation, IncompleteDetails, Instructions, OutputItem, Prompt,
    PromptCacheRetention, Reasoning, Response, ResponseTextParam, ResponseUsage, Status, Tool,
    ToolChoiceParam, Truncation,
};

/// Protocol-focused OpenAI Responses response projection.
///
/// Field order follows the OpenAI Responses response schema for the
/// fields we intentionally retain.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResponseProjection {
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

impl From<&Response> for ResponseProjection {
    fn from(response: &Response) -> Self {
        Self {
            background: response.background,
            billing: response.billing.clone(),
            conversation: response.conversation.clone(),
            created_at: response.created_at,
            completed_at: response.completed_at,
            error: response.error.clone(),
            id: response.id.clone(),
            incomplete_details: response.incomplete_details.clone(),
            instructions: response.instructions.clone(),
            max_output_tokens: response.max_output_tokens,
            metadata: response.metadata.clone(),
            model: response.model.clone(),
            object: response.object.clone(),
            output: response.output.clone(),
            parallel_tool_calls: response.parallel_tool_calls,
            previous_response_id: response.previous_response_id.clone(),
            prompt: response.prompt.clone(),
            prompt_cache_key: response.prompt_cache_key.clone(),
            prompt_cache_retention: response.prompt_cache_retention,
            reasoning: response.reasoning.clone(),
            safety_identifier: response.safety_identifier.clone(),
            service_tier: response.service_tier,
            status: response.status,
            temperature: response.temperature,
            text: response.text.clone(),
            tool_choice: response.tool_choice.clone(),
            tools: response.tools.clone(),
            top_logprobs: response.top_logprobs,
            top_p: response.top_p,
            truncation: response.truncation,
            usage: response.usage,
        }
    }
}
