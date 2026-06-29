use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use serde_json::Value;

use super::super::ServiceTier;
use super::super::wire::CreateChatCompletionRequest;
use super::{
    ChatCompletionAudio, ChatCompletionStreamOptions, ChatCompletionToolChoiceOption,
    ChatCompletionTools, PredictionContent, ReasoningEffort, ResponseFormat, ResponseModalities,
    StopConfiguration, Verbosity, WebSearchOptions,
};

/// Protocol-focused OpenAI Chat Completions request projection.
///
/// Field order follows the OpenAI Chat Completions request schema
/// for the fields we intentionally retain.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RequestProjection {
    // Intentionally omitted for now: `messages`.
    // Messages can be very large and private; request hints should not depend on
    // raw prompt body contents.
    pub model: Option<String>,
    pub modalities: Option<Vec<ResponseModalities>>,
    pub verbosity: Option<Verbosity>,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub max_completion_tokens: Option<u32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub web_search_options: Option<WebSearchOptions>,
    pub top_logprobs: Option<u8>,
    pub response_format: Option<ResponseFormat>,
    pub audio: Option<ChatCompletionAudio>,
    pub store: Option<bool>,
    pub stream: Option<bool>,
    pub stop: Option<StopConfiguration>,
    pub logit_bias: Option<HashMap<String, i8>>,
    pub logprobs: Option<bool>,
    pub max_tokens: Option<u32>,
    pub n: Option<u8>,
    pub prediction: Option<PredictionContent>,
    // Deprecated upstream: `seed`.
    pub stream_options: Option<ChatCompletionStreamOptions>,
    pub service_tier: Option<ServiceTier>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub tools: Option<Vec<ChatCompletionTools>>,
    pub tool_choice: Option<ChatCompletionToolChoiceOption>,
    pub parallel_tool_calls: Option<bool>,
    // Deprecated upstream: `user`.
    pub safety_identifier: Option<String>,
    pub prompt_cache_key: Option<String>,
    // Deprecated upstream: `function_call`, `functions`.
    pub metadata: Option<HashMap<String, String>>,
}

impl RequestProjection {
    pub fn from_payload(payload: &Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value::<CreateChatCompletionRequest>(payload.clone()).map(Into::into)
    }
}

impl From<CreateChatCompletionRequest> for RequestProjection {
    fn from(request: CreateChatCompletionRequest) -> Self {
        #[allow(deprecated)]
        let max_tokens = request.max_tokens;
        let prompt_cache_key = request
            .prompt_cache_key
            .filter(|value| !value.trim().is_empty());
        Self {
            model: Some(request.model),
            modalities: request.modalities,
            verbosity: request.verbosity,
            reasoning_effort: request.reasoning_effort,
            max_completion_tokens: request.max_completion_tokens,
            frequency_penalty: request.frequency_penalty,
            presence_penalty: request.presence_penalty,
            web_search_options: request.web_search_options,
            top_logprobs: request.top_logprobs,
            response_format: request.response_format,
            audio: request.audio,
            store: request.store,
            stream: request.stream,
            stop: request.stop,
            logit_bias: request.logit_bias,
            logprobs: request.logprobs,
            max_tokens,
            n: request.n,
            prediction: request.prediction,
            stream_options: request.stream_options,
            service_tier: request.service_tier,
            temperature: request.temperature,
            top_p: request.top_p,
            tools: request.tools,
            tool_choice: request.tool_choice,
            parallel_tool_calls: request.parallel_tool_calls,
            safety_identifier: request.safety_identifier,
            prompt_cache_key,
            metadata: request.metadata,
        }
    }
}
