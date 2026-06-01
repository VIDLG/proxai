use crate::protocol::anthropic::messages::ThinkingConfigParam;
use crate::provider::ForwardedRequestView;
use serde_json::Value as JsonValue;
use valuable::Valuable;
use valuable_serde::Serializable;

use super::request_hints;
use super::ForwardedRequestEvent;

#[derive(Debug, Clone, Valuable)]
pub(crate) struct ForwardFields {
    pub(crate) request_id: u64,
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) forwarded_request_bytes: u64,
    pub(crate) inbound_request_bytes: u64,
    pub(crate) request_delta_bytes: i128,
    pub(crate) model: String,
    pub(crate) reasoning_effort: String,
    pub(crate) stream: Option<bool>,
    pub(crate) max_output_tokens: Option<u32>,
    pub(crate) request_hints: String,
    pub(crate) capture: bool,
}

impl From<&ForwardedRequestEvent<'_>> for ForwardFields {
    fn from(event: &ForwardedRequestEvent<'_>) -> Self {
        let mut hint_parts = Vec::new();
        let projection_hints = request_hints::render_projection_compact(&event.forwarded_request);
        if !projection_hints.is_empty() {
            hint_parts.push(projection_hints);
        }
        hint_parts.extend(request_hints::render_summary_compact(
            &event.forwarded_request,
        ));
        let common = ForwardRequestCommonFields::from(&event.forwarded_request);

        Self {
            request_id: event.request_id,
            method: event.method.to_string(),
            path: event.uri.to_string(),
            forwarded_request_bytes: event.request_sizes.forwarded,
            inbound_request_bytes: event.request_sizes.inbound,
            request_delta_bytes: event.request_sizes.delta(),
            model: common.model,
            reasoning_effort: common.reasoning_effort,
            stream: common.stream,
            max_output_tokens: common.max_output_tokens,
            request_hints: hint_parts.join(" "),
            capture: event.capture,
        }
    }
}

struct ForwardRequestCommonFields {
    model: String,
    reasoning_effort: String,
    stream: Option<bool>,
    max_output_tokens: Option<u32>,
}

impl From<&ForwardedRequestView<'_>> for ForwardRequestCommonFields {
    fn from(forwarded_request: &ForwardedRequestView<'_>) -> Self {
        match forwarded_request {
            ForwardedRequestView::OpenaiResponses {
                projection,
                summary: _,
            } => Self {
                model: projection.model.clone().unwrap_or_default(),
                reasoning_effort: projection
                    .reasoning
                    .as_ref()
                    .and_then(|value| value.effort)
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                stream: projection.stream,
                max_output_tokens: projection.max_output_tokens,
            },
            ForwardedRequestView::OpenaiChatCompletions {
                projection,
                summary: _,
            } => Self {
                model: projection.model.clone().unwrap_or_default(),
                reasoning_effort: projection
                    .reasoning_effort
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                stream: projection.stream,
                max_output_tokens: projection.max_completion_tokens.or(projection.max_tokens),
            },
            ForwardedRequestView::AnthropicMessages {
                projection,
                summary: _,
            } => Self {
                model: projection.model.clone(),
                reasoning_effort: projection
                    .thinking
                    .as_ref()
                    .map(render_anthropic_thinking_common_field)
                    .unwrap_or_default(),
                stream: projection.stream,
                max_output_tokens: Some(projection.max_tokens),
            },
        }
    }
}

fn render_anthropic_thinking_common_field(thinking: &ThinkingConfigParam) -> String {
    match thinking {
        ThinkingConfigParam::Enabled(value) => value.budget_tokens.to_string(),
        ThinkingConfigParam::Adaptive(_) => "adaptive".to_string(),
        ThinkingConfigParam::Disabled(_) => "disabled".to_string(),
    }
}

pub(crate) trait ValuableJson {
    fn to_json_value(&self) -> JsonValue;
}

impl ValuableJson for ForwardFields {
    fn to_json_value(&self) -> JsonValue {
        serde_json::to_value(Serializable::new(self)).unwrap_or(JsonValue::Null)
    }
}
