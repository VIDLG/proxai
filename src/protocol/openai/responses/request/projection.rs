use async_openai::types::responses as openai;
use async_openai::types::responses::CreateResponse;
use std::collections::HashMap;
use structural_convert::StructuralConvert;

use super::super::wire::{
    Conversation, IncludeEnum, Prompt, PromptCacheRetention, Reasoning, ResponseStreamOptions,
    ResponseTextParam, ServiceTier, Tool, ToolChoiceParam, Truncation,
};

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ConversationParam))]
pub enum ConversationParam {
    ConversationID(String),
    Object(Conversation),
}

/// Protocol-focused OpenAI Responses request projection.
///
/// Field order follows `async_openai::types::responses::CreateResponse` for the
/// fields we intentionally retain.
#[derive(Debug, Clone, Default, PartialEq)]
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

impl From<CreateResponse> for RequestProjection {
    fn from(request: CreateResponse) -> Self {
        let previous_response_id = request
            .previous_response_id
            .filter(|value| !value.trim().is_empty());
        let prompt_cache_key = request
            .prompt_cache_key
            .filter(|value| !value.trim().is_empty());

        Self {
            background: request.background,
            conversation: request.conversation.map(Into::into),
            include: request
                .include
                .map(|values| values.into_iter().map(Into::into).collect()),
            instructions: request.instructions,
            max_output_tokens: request.max_output_tokens,
            max_tool_calls: request.max_tool_calls,
            metadata: request.metadata,
            model: request.model,
            parallel_tool_calls: request.parallel_tool_calls,
            previous_response_id,
            prompt: request.prompt.clone().map(Into::into),
            prompt_cache_key,
            prompt_cache_retention: request.prompt_cache_retention.map(Into::into),
            reasoning: request.reasoning.clone().map(Into::into),
            safety_identifier: request.safety_identifier,
            service_tier: request.service_tier.map(Into::into),
            store: request.store,
            stream: request.stream,
            stream_options: request.stream_options.map(Into::into),
            temperature: request.temperature,
            text: request.text.clone().map(Into::into),
            tool_choice: request.tool_choice.map(Into::into),
            tools: request
                .tools
                .map(|values| values.into_iter().map(Into::into).collect()),
            top_logprobs: request.top_logprobs,
            top_p: request.top_p,
            truncation: request.truncation.map(Into::into),
        }
    }
}
