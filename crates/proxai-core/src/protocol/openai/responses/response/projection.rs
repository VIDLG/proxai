//! OpenAI Responses protocol-native response projection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::protocol::ErrorObject;

use super::super::wire::ServiceTier;

use super::super::wire::{
    Conversation, IncompleteDetails, Instructions, OutputItem, Prompt, PromptCacheRetention,
    Reasoning, Response, ResponseTextParam, ResponseUsage, Status, Tool, ToolChoiceParam,
    Truncation,
};

/// Protocol-focused OpenAI Responses response projection.
///
/// Field order follows the OpenAI Responses response schema for the
/// fields we intentionally retain.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResponseProjection {
    pub background: Option<bool>,
    pub conversation: Option<Conversation>,
    pub created_at: f64,
    pub completed_at: Option<f64>,
    pub error: Option<ErrorObject>,
    pub id: String,
    pub incomplete_details: Option<IncompleteDetails>,
    pub instructions: Option<Instructions>,
    pub max_output_tokens: Option<u32>,
    pub max_tool_calls: Option<u32>,
    pub metadata: Option<HashMap<String, String>>,
    pub model: String,
    pub object: String,
    pub output: Vec<OutputItem>,
    pub output_text: Option<String>,
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
    pub user: Option<String>,
    pub usage: Option<ResponseUsage>,
}

impl From<&Response> for ResponseProjection {
    fn from(response: &Response) -> Self {
        Self {
            background: response.background.into_non_null(),
            conversation: response.conversation.clone().into_non_null(),
            created_at: response.created_at,
            completed_at: response.completed_at.into_non_null(),
            error: response.error.as_non_null().map(|error| ErrorObject {
                code: error.code.to_string(),
                message: error.message.clone(),
            }),
            id: response.id.clone(),
            incomplete_details: response.incomplete_details.as_non_null().cloned(),
            instructions: response.instructions.as_non_null().cloned(),
            max_output_tokens: response.max_output_tokens.into_non_null(),
            max_tool_calls: response.max_tool_calls.into_non_null(),
            metadata: response.metadata.as_non_null().cloned(),
            model: response.model.clone(),
            object: response.object.to_string(),
            output: response.output.clone(),
            output_text: response.output_text.clone().into_non_null(),
            parallel_tool_calls: Some(response.parallel_tool_calls),
            previous_response_id: response.previous_response_id.clone().into_non_null(),
            prompt: response.prompt.clone().into_non_null(),
            prompt_cache_key: response.prompt_cache_key.clone(),
            prompt_cache_retention: response.prompt_cache_retention.into_non_null(),
            reasoning: response.reasoning.clone().into_non_null(),
            safety_identifier: response.safety_identifier.clone(),
            service_tier: response.service_tier.into_non_null(),
            status: response.status.unwrap_or_default(),
            temperature: response.temperature.as_non_null().copied(),
            text: response.text.clone(),
            tool_choice: Some(response.tool_choice.clone()),
            tools: Some(response.tools.clone()),
            top_logprobs: response.top_logprobs.into_non_null(),
            top_p: response.top_p.as_non_null().copied(),
            truncation: response.truncation.into_non_null(),
            user: response.user.clone(),
            usage: response.usage,
        }
    }
}
