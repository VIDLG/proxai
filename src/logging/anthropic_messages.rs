use serde_json::Value as JsonValue;
use tracing::{info, warn};
use valuable::Valuable;
use valuable_serde::Serializable;

use crate::config::LogOutputFormat;
use crate::formatting::{compact_tail, format_count_map};

use crate::provider::anthropic_messages::AnthropicUpstreamResponseSnapshot;

use super::record::ValuableJson;
use super::{active_log_format, emit_json_log, extend_json_object, rename_json_field};

#[derive(Debug, Clone, Default, Valuable)]
struct AnthropicResponseFields {
    id: String,
    model: String,
    service_tier: String,
    stop_reason: String,
    tok: u32,
    input: u32,
    cache_read: Option<u32>,
    cache_creation: Option<u32>,
    output: u32,
    output_items: String,
    stop_reasons: String,
    tool_uses: String,
    server_tool_uses: String,
}

impl From<&AnthropicUpstreamResponseSnapshot> for AnthropicResponseFields {
    fn from(snapshot: &AnthropicUpstreamResponseSnapshot) -> Self {
        let projection = &snapshot.state.projection;
        let input = projection.input_tokens().unwrap_or_default();
        let cache_read = projection.cache_read_input_tokens();
        let cache_creation = projection.cache_creation_input_tokens();
        let output = projection.output_tokens().unwrap_or_default();

        Self {
            id: compact_tail(projection.id().as_deref().unwrap_or_default(), 8),
            model: projection.model().clone().unwrap_or_default(),
            service_tier: projection
                .service_tier()
                .map(|service_tier| service_tier.to_string())
                .unwrap_or_default(),
            stop_reason: projection
                .stop_reason()
                .map(|reason| reason.to_string())
                .unwrap_or_default(),
            tok: input.saturating_add(output),
            input,
            cache_read,
            cache_creation,
            output,
            output_items: format_count_map(&snapshot.state.summary.output_items),
            stop_reasons: format_count_map(&snapshot.state.summary.stop_reasons),
            tool_uses: snapshot
                .state
                .summary
                .tool_uses
                .iter()
                .map(|(key, value)| format!("{}:{value}", super::compact_tool_call_name(key)))
                .collect::<Vec<_>>()
                .join(" "),
            server_tool_uses: format_count_map(&snapshot.state.summary.server_tool_uses),
        }
    }
}

impl ValuableJson for AnthropicResponseFields {
    fn to_json_value(&self) -> JsonValue {
        serde_json::to_value(Serializable::new(self)).unwrap_or(JsonValue::Null)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum AnthropicLogRecord<'a> {
    Completed {
        snapshot: &'a AnthropicUpstreamResponseSnapshot,
    },
    Closed {
        snapshot: &'a AnthropicUpstreamResponseSnapshot,
    },
    StreamError {
        snapshot: &'a AnthropicUpstreamResponseSnapshot,
        message: &'a str,
    },
}

impl AnthropicLogRecord<'_> {
    pub(crate) fn emit(self) {
        match self {
            Self::Completed { snapshot } => emit_anthropic_stream_info("end", snapshot),
            Self::Closed { snapshot } => emit_anthropic_stream_info("closed", snapshot),
            Self::StreamError { snapshot, message } => {
                emit_anthropic_stream_error(snapshot, message)
            }
        }
    }
}

fn emit_anthropic_stream_info(event: &str, snapshot: &AnthropicUpstreamResponseSnapshot) {
    let head = &snapshot.head;
    let response = AnthropicResponseFields::from(snapshot);

    match active_log_format() {
        LogOutputFormat::Human => info!(
            event = event,
            status = head.status.as_u16(),
            ttfb_ms = head.ttfb.as_millis() as u64,
            down = snapshot.metrics.bytes,
            chunks = snapshot.metrics.chunks,
            avg_chunk_bytes = snapshot.metrics.avg_chunk_bytes(),
            duration_ms = snapshot.metrics.duration_ms(),
            ct = head
                .content_type
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            sse = head.is_sse(),
            response_id = response.id,
            model = response.model,
            service_tier = response.service_tier,
            stop_reason = response.stop_reason,
            tok = response.tok,
            input = response.input,
            cache_read = response.cache_read,
            cache_creation = response.cache_creation,
            output = response.output,
            output_items = response.output_items,
            stop_reasons = response.stop_reasons,
            tool_uses = response.tool_uses,
            server_tool_uses = response.server_tool_uses,
        ),
        LogOutputFormat::Json => {
            let mut payload = response.to_json_value();
            rename_json_field(&mut payload, "id", "response_id");
            extend_json_object(
                &mut payload,
                [
                    ("status", JsonValue::from(head.status.as_u16())),
                    ("ttfb_ms", JsonValue::from(head.ttfb.as_millis() as u64)),
                    ("down", JsonValue::from(snapshot.metrics.bytes)),
                    ("chunks", JsonValue::from(snapshot.metrics.chunks)),
                    (
                        "avg_chunk_bytes",
                        JsonValue::from(snapshot.metrics.avg_chunk_bytes()),
                    ),
                    (
                        "duration_ms",
                        JsonValue::from(snapshot.metrics.duration_ms()),
                    ),
                    (
                        "ct",
                        JsonValue::String(
                            head.content_type
                                .as_ref()
                                .map(ToString::to_string)
                                .unwrap_or_default(),
                        ),
                    ),
                    ("sse", JsonValue::Bool(head.is_sse())),
                ],
            );
            emit_json_log("INFO", event, payload);
        }
    }
}

fn emit_anthropic_stream_error(snapshot: &AnthropicUpstreamResponseSnapshot, message: &str) {
    let head = &snapshot.head;
    let response = AnthropicResponseFields::from(snapshot);

    match active_log_format() {
        LogOutputFormat::Human => warn!(
            event = "stream-error",
            status = head.status.as_u16(),
            ttfb_ms = head.ttfb.as_millis() as u64,
            down = snapshot.metrics.bytes,
            chunks = snapshot.metrics.chunks,
            avg_chunk_bytes = snapshot.metrics.avg_chunk_bytes(),
            duration_ms = snapshot.metrics.duration_ms(),
            ct = head
                .content_type
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            sse = head.is_sse(),
            response_id = response.id,
            model = response.model,
            service_tier = response.service_tier,
            stop_reason = response.stop_reason,
            tok = response.tok,
            input = response.input,
            cache_read = response.cache_read,
            cache_creation = response.cache_creation,
            output = response.output,
            output_items = response.output_items,
            stop_reasons = response.stop_reasons,
            tool_uses = response.tool_uses,
            server_tool_uses = response.server_tool_uses,
            err = message,
        ),
        LogOutputFormat::Json => {
            let mut payload = response.to_json_value();
            rename_json_field(&mut payload, "id", "response_id");
            extend_json_object(
                &mut payload,
                [
                    ("status", JsonValue::from(head.status.as_u16())),
                    ("ttfb_ms", JsonValue::from(head.ttfb.as_millis() as u64)),
                    ("down", JsonValue::from(snapshot.metrics.bytes)),
                    ("chunks", JsonValue::from(snapshot.metrics.chunks)),
                    (
                        "avg_chunk_bytes",
                        JsonValue::from(snapshot.metrics.avg_chunk_bytes()),
                    ),
                    (
                        "duration_ms",
                        JsonValue::from(snapshot.metrics.duration_ms()),
                    ),
                    (
                        "ct",
                        JsonValue::String(
                            head.content_type
                                .as_ref()
                                .map(ToString::to_string)
                                .unwrap_or_default(),
                        ),
                    ),
                    ("sse", JsonValue::Bool(head.is_sse())),
                    ("err", JsonValue::String(message.to_string())),
                ],
            );
            emit_json_log("WARN", "stream-error", payload);
        }
    }
}
