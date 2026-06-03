use std::collections::BTreeMap;

use serde_json::Value as JsonValue;
use tracing::{info, warn};
use valuable::Valuable;

use crate::config::LogOutputFormat;
use crate::formatting::{compact_tail, format_count_map};

use crate::provider::anthropic_messages::{
    AnthropicResponseOutputKind, AnthropicUpstreamResponseSnapshot,
};
use crate::upstream::UpstreamStreamError;

use super::counts::{
    compact_output_items_for_human, compact_tool_calls, full_count_map, join_call_maps,
    merge_count_maps, source_count_maps, string_count_map,
};
use super::record::ValuableJson;
use super::upstream::{stream_error_text, stream_error_token};
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
    output_items: BTreeMap<String, u64>,
    output_items_human: String,
    stop_reasons: BTreeMap<String, u64>,
    calls: BTreeMap<String, u64>,
    calls_by_source: BTreeMap<String, BTreeMap<String, u64>>,
    calls_human: String,
}

impl From<&AnthropicUpstreamResponseSnapshot> for AnthropicResponseFields {
    fn from(snapshot: &AnthropicUpstreamResponseSnapshot) -> Self {
        let state = &snapshot.state;
        let input = state.input_tokens().unwrap_or_default();
        let cache_read = state.cache_read_input_tokens();
        let cache_creation = state.cache_creation_input_tokens();
        let output = state.output_tokens().unwrap_or_default();
        let tool_uses = string_count_map(&snapshot.state.summary.tool_uses);
        let server_tool_uses = string_count_map(&snapshot.state.summary.server_tool_uses);

        Self {
            id: compact_tail(state.id().as_deref().unwrap_or_default(), 8),
            model: state.model().clone().unwrap_or_default(),
            service_tier: state
                .service_tier()
                .map(|service_tier| service_tier.to_string())
                .unwrap_or_default(),
            stop_reason: state
                .stop_reason()
                .map(|reason| reason.to_string())
                .unwrap_or_default(),
            tok: input.saturating_add(output),
            input,
            cache_read,
            cache_creation,
            output,
            output_items: string_count_map(&snapshot.state.summary.output_items),
            output_items_human: compact_output_items_for_human(
                &snapshot.state.summary.output_items,
                AnthropicResponseOutputKind::Text,
            ),
            stop_reasons: string_count_map(&snapshot.state.summary.stop_reasons),
            calls: merge_count_maps([tool_uses.clone(), server_tool_uses.clone()]),
            calls_by_source: source_count_maps([
                ("tool", tool_uses),
                ("server_tool", server_tool_uses),
            ]),
            calls_human: join_call_maps([
                compact_tool_calls(&snapshot.state.summary.tool_uses),
                full_count_map(&snapshot.state.summary.server_tool_uses),
            ]),
        }
    }
}

impl ValuableJson for AnthropicResponseFields {
    fn to_json_value(&self) -> JsonValue {
        super::json_object([
            ("id", JsonValue::String(self.id.clone())),
            ("model", JsonValue::String(self.model.clone())),
            ("service_tier", JsonValue::String(self.service_tier.clone())),
            ("stop_reason", JsonValue::String(self.stop_reason.clone())),
            ("tok", JsonValue::from(self.tok)),
            ("input", JsonValue::from(self.input)),
            (
                "cache_read",
                self.cache_read
                    .map(JsonValue::from)
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "cache_creation",
                self.cache_creation
                    .map(JsonValue::from)
                    .unwrap_or(JsonValue::Null),
            ),
            ("output", JsonValue::from(self.output)),
            (
                "output_items",
                serde_json::to_value(&self.output_items).unwrap_or(JsonValue::Null),
            ),
            (
                "stop_reasons",
                serde_json::to_value(&self.stop_reasons).unwrap_or(JsonValue::Null),
            ),
            (
                "calls",
                serde_json::to_value(&self.calls).unwrap_or(JsonValue::Null),
            ),
            (
                "calls_by_source",
                serde_json::to_value(&self.calls_by_source).unwrap_or(JsonValue::Null),
            ),
        ])
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
        error: &'a UpstreamStreamError,
    },
}

impl AnthropicLogRecord<'_> {
    pub(crate) fn emit(self) {
        match self {
            Self::Completed { snapshot } => emit_anthropic_stream_info("end", snapshot),
            Self::Closed { snapshot } => emit_anthropic_stream_info("closed", snapshot),
            Self::StreamError { snapshot, error } => emit_anthropic_stream_error(snapshot, error),
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
            ct = head.content_type_text(),
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
            output_items_human = response.output_items_human,
            stop_reasons = format_count_map(&response.stop_reasons),
            calls_human = response.calls_human,
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
                    ("ct", JsonValue::String(head.content_type_text())),
                    ("sse", JsonValue::Bool(head.is_sse())),
                ],
            );
            emit_json_log("INFO", event, payload);
        }
    }
}

fn emit_anthropic_stream_error(
    snapshot: &AnthropicUpstreamResponseSnapshot,
    error: &UpstreamStreamError,
) {
    let head = &snapshot.head;
    let response = AnthropicResponseFields::from(snapshot);

    match active_log_format() {
        LogOutputFormat::Human => warn!(
            event = stream_error_token(error),
            status = head.status.as_u16(),
            ttfb_ms = head.ttfb.as_millis() as u64,
            down = snapshot.metrics.bytes,
            chunks = snapshot.metrics.chunks,
            avg_chunk_bytes = snapshot.metrics.avg_chunk_bytes(),
            duration_ms = snapshot.metrics.duration_ms(),
            ct = head.content_type_text(),
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
            output_items_human = response.output_items_human,
            stop_reasons = format_count_map(&response.stop_reasons),
            calls_human = response.calls_human,
            err = stream_error_text(error),
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
                    ("ct", JsonValue::String(head.content_type_text())),
                    ("sse", JsonValue::Bool(head.is_sse())),
                    ("err", JsonValue::String(stream_error_text(error))),
                ],
            );
            emit_json_log("WARN", stream_error_token(error), payload);
        }
    }
}

#[cfg(test)]
#[path = "anthropic_messages_tests.rs"]
mod tests;
