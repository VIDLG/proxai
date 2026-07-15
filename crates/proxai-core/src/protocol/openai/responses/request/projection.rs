use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::super::wire::{
    ConversationParam, IncludeEnum, Prompt, PromptCacheRetention, Reasoning, ResponseStreamOptions,
    ResponseTextParam, ServiceTier, Tool, ToolChoiceParam, Truncation,
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct ResponseProjectionPayload {
    pub background: Option<bool>,
    pub conversation: Option<ConversationParam>,
    pub include: Option<Vec<IncludeEnum>>,
    pub instructions: Option<String>,
    pub max_output_tokens: Option<u32>,
    pub max_tool_calls: Option<u32>,
    pub metadata: Option<HashMap<String, String>>,
    pub model: Option<String>,
    pub parallel_tool_calls: Option<bool>,
    pub previous_response_id: Option<String>,
    pub prompt: Option<Prompt>,
    pub prompt_cache_key: Option<String>,
    pub prompt_cache_retention: Option<PromptCacheRetention>,
    pub reasoning: Option<Reasoning>,
    pub safety_identifier: Option<String>,
    pub service_tier: Option<ServiceTier>,
    pub store: Option<bool>,
    pub stream: Option<bool>,
    pub stream_options: Option<ResponseStreamOptions>,
    pub temperature: Option<f32>,
    pub text: Option<ResponseTextParam>,
    pub tool_choice: Option<ToolChoiceParam>,
    pub tools: Option<Vec<Tool>>,
    pub top_logprobs: Option<u8>,
    pub top_p: Option<f32>,
    pub truncation: Option<Truncation>,
}

/// Protocol-focused OpenAI Responses request projection.
///
/// Field order follows the OpenAI Responses request schema for the
/// fields we intentionally retain.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RequestProjection {
    pub background: Option<bool>,
    pub conversation: Option<ConversationParam>,
    pub include: Option<Vec<IncludeEnum>>,
    // Intentionally omitted for now: `input`.
    // Input can be very large, and downstream code should not depend on raw
    // request body contents through the projection layer.
    pub instructions: Option<String>,
    pub max_output_tokens: Option<u32>,
    pub max_tool_calls: Option<u32>,
    pub metadata: Option<HashMap<String, String>>,
    pub model: Option<String>,
    pub parallel_tool_calls: Option<bool>,
    pub previous_response_id: Option<String>,
    pub prompt: Option<Prompt>,
    pub prompt_cache_key: Option<String>,
    pub prompt_cache_retention: Option<PromptCacheRetention>,
    pub reasoning: Option<Reasoning>,
    pub safety_identifier: Option<String>,
    pub service_tier: Option<ServiceTier>,
    pub store: Option<bool>,
    pub stream: Option<bool>,
    pub stream_options: Option<ResponseStreamOptions>,
    pub temperature: Option<f32>,
    pub text: Option<ResponseTextParam>,
    pub tool_choice: Option<ToolChoiceParam>,
    pub tools: Option<Vec<Tool>>,
    pub top_logprobs: Option<u8>,
    pub top_p: Option<f32>,
    pub truncation: Option<Truncation>,
}

impl RequestProjection {
    pub fn from_payload(payload: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value::<ResponseProjectionPayload>(payload.clone()).map(Into::into)
    }
}

impl From<ResponseProjectionPayload> for RequestProjection {
    fn from(request: ResponseProjectionPayload) -> Self {
        let previous_response_id = request
            .previous_response_id
            .filter(|value| !value.trim().is_empty());
        let prompt_cache_key = request
            .prompt_cache_key
            .filter(|value| !value.trim().is_empty());

        Self {
            background: request.background,
            conversation: request.conversation,
            include: request.include,
            instructions: request.instructions,
            max_output_tokens: request.max_output_tokens,
            max_tool_calls: request.max_tool_calls,
            metadata: request.metadata,
            model: request.model,
            parallel_tool_calls: request.parallel_tool_calls,
            previous_response_id,
            prompt: request.prompt,
            prompt_cache_key,
            prompt_cache_retention: request.prompt_cache_retention,
            reasoning: request.reasoning,
            safety_identifier: request.safety_identifier,
            service_tier: request.service_tier,
            store: request.store,
            stream: request.stream,
            stream_options: request.stream_options,
            temperature: request.temperature,
            text: request.text,
            tool_choice: request.tool_choice,
            tools: request.tools,
            top_logprobs: request.top_logprobs,
            top_p: request.top_p,
            truncation: request.truncation,
        }
    }
}

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
