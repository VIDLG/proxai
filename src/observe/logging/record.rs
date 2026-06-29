use crate::protocol::anthropic::messages::{OutputEffort, ThinkingConfigParam};
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::provider::ProviderRequestView;
use crate::request::RequestId;
use serde_json::{Value as JsonValue, json};
use valuable::Valuable;

use super::ProviderRequestLogPayload;
use super::request_hints;

#[derive(Debug, Clone, Valuable)]
pub(crate) struct ProviderRequestFields {
    pub(crate) request_id: RequestId,
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) provider_request_bytes: usize,
    pub(crate) inbound_request_bytes: usize,
    pub(crate) request_delta_bytes: i128,
    pub(crate) request_protocol: String,
    pub(crate) provider: String,
    pub(crate) route_name: Option<String>,
    pub(crate) provider_protocol: String,
    pub(crate) translation: String,
    pub(crate) request_protocol_alias: String,
    pub(crate) translation_alias: String,
    pub(crate) provider_protocol_alias: String,
    pub(crate) model: String,
    pub(crate) reasoning_effort: String,
    pub(crate) stream: Option<bool>,
    pub(crate) max_output_tokens: Option<u32>,
    pub(crate) request_hints: String,
    pub(crate) request_hint_parts: Vec<String>,
    pub(crate) capture: bool,
}

impl From<&ProviderRequestLogPayload<'_>> for ProviderRequestFields {
    fn from(event: &ProviderRequestLogPayload<'_>) -> Self {
        let mut hint_parts = Vec::new();
        let projection_hints = request_hints::render_projection_compact(&event.provider_request);
        if !projection_hints.is_empty() {
            hint_parts.push(projection_hints);
        }
        hint_parts.extend(request_hints::render_summary_compact(
            &event.provider_request,
        ));
        let common = ProviderRequestCommonFields::from(&event.provider_request);

        let request_hints = hint_parts.join(" ");

        Self {
            request_id: event.request_id,
            method: event.method.to_string(),
            path: event.uri.to_string(),
            provider_request_bytes: event.request_sizes.provider,
            inbound_request_bytes: event.request_sizes.inbound,
            request_delta_bytes: event.request_sizes.delta(),
            request_protocol: event.request_protocol.to_string(),
            provider: event.provider.clone(),
            route_name: event.route_name.clone(),
            provider_protocol: event.provider_protocol.to_string(),
            translation: render_translation(event.request_protocol, event.provider_protocol),
            request_protocol_alias: compact_request_protocol(event.request_protocol).to_string(),
            translation_alias: render_translation_alias(
                event.request_protocol,
                event.provider_protocol,
            ),
            provider_protocol_alias: compact_provider_protocol(event.provider_protocol).to_string(),
            model: common.model,
            reasoning_effort: common.reasoning_effort,
            stream: common.stream,
            max_output_tokens: common.max_output_tokens,
            request_hints,
            request_hint_parts: hint_parts,
            capture: event.capture,
        }
    }
}

fn render_translation(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
) -> String {
    if request_protocol.matches_provider_protocol(provider_protocol) {
        String::new()
    } else {
        format!("{request_protocol}->{provider_protocol}")
    }
}

fn render_translation_alias(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
) -> String {
    if request_protocol.matches_provider_protocol(provider_protocol) {
        String::new()
    } else {
        format!(
            "{}->{}",
            compact_request_protocol(request_protocol),
            compact_provider_protocol(provider_protocol)
        )
    }
}

fn compact_request_protocol(protocol: RequestProtocol) -> &'static str {
    match protocol {
        RequestProtocol::OpenaiResponses => "resp",
        RequestProtocol::OpenaiChatCompletions => "chat",
        RequestProtocol::AnthropicMessages => "ant",
    }
}

fn compact_provider_protocol(protocol: ProviderProtocol) -> &'static str {
    match protocol {
        ProviderProtocol::OpenaiResponses => "resp",
        ProviderProtocol::OpenaiChatCompletions => "chat",
        ProviderProtocol::AnthropicMessages => "ant",
    }
}

struct ProviderRequestCommonFields {
    model: String,
    reasoning_effort: String,
    stream: Option<bool>,
    max_output_tokens: Option<u32>,
}

impl From<&ProviderRequestView<'_>> for ProviderRequestCommonFields {
    fn from(provider_request: &ProviderRequestView<'_>) -> Self {
        match provider_request {
            ProviderRequestView::OpenaiResponses {
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
            ProviderRequestView::OpenaiChatCompletions {
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
            ProviderRequestView::AnthropicMessages {
                projection,
                summary: _,
            } => Self {
                model: projection.model.clone(),
                reasoning_effort: projection
                    .output_config
                    .as_ref()
                    .and_then(|config| config.effort)
                    .map(render_anthropic_output_effort_common_field)
                    .or_else(|| {
                        projection
                            .thinking
                            .as_ref()
                            .map(render_anthropic_thinking_common_field)
                    })
                    .unwrap_or_default(),
                stream: projection.stream,
                max_output_tokens: Some(projection.max_tokens),
            },
        }
    }
}

fn render_anthropic_output_effort_common_field(effort: OutputEffort) -> String {
    match effort {
        OutputEffort::Low => "low",
        OutputEffort::Medium => "medium",
        OutputEffort::High => "high",
        OutputEffort::Xhigh => "xhigh",
        OutputEffort::Max => "max",
    }
    .to_string()
}

fn render_anthropic_thinking_common_field(thinking: &ThinkingConfigParam) -> String {
    match thinking {
        ThinkingConfigParam::Enabled(_) => "legacy_enabled".to_string(),
        ThinkingConfigParam::Adaptive(_) => "adaptive".to_string(),
        ThinkingConfigParam::Disabled(_) => "disabled".to_string(),
    }
}

pub(crate) trait ValuableJson {
    fn to_json_value(&self) -> JsonValue;
}

impl ValuableJson for ProviderRequestFields {
    fn to_json_value(&self) -> JsonValue {
        json!({
            "request_id": self.request_id,
            "method": self.method,
            "path": self.path,
            "provider_request_bytes": self.provider_request_bytes,
            "inbound_request_bytes": self.inbound_request_bytes,
            "request_delta_bytes": self.request_delta_bytes,
            "request_protocol": self.request_protocol,
            "provider": self.provider,
            "route_name": self.route_name,
            "provider_protocol": self.provider_protocol,
            "translation": self.translation,
            "request_protocol_alias": self.request_protocol_alias,
            "translation_alias": self.translation_alias,
            "provider_protocol_alias": self.provider_protocol_alias,
            "model": self.model,
            "reasoning_effort": self.reasoning_effort,
            "stream": self.stream,
            "max_output_tokens": self.max_output_tokens,
            "request_hints": self.request_hints,
            "request_hint_parts": self.request_hint_parts,
            "capture": self.capture,
        })
    }
}
