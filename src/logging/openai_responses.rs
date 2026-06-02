use std::collections::BTreeMap;

use serde_json::Value as JsonValue;
use tracing::{info, warn};
use valuable::Valuable;

use crate::config::LogOutputFormat;
use crate::formatting::{compact_tail, truncate_chars};
use crate::provider::openai::responses::{
    ResponseOutputItemKind, ResponsesUpstreamEvent, ResponsesUpstreamState,
    ResponsesUpstreamStreamSnapshot,
};
use crate::provider::UpstreamResponseError;
use crate::upstream::ContentType;

use super::counts::{
    compact_output_items_for_human, compact_tool_calls, full_count_map, join_call_maps,
    merge_count_maps, source_count_maps, string_count_map,
};
use super::record::ValuableJson;
use super::upstream::{error_text, error_token};
use super::{
    active_log_format, emit_json_log, extend_json_object, optional_f64, optional_u64,
    rename_json_field, UpstreamLogRecord,
};

#[derive(Debug, Clone, Default, Valuable)]
struct ResponseFields {
    id: String,
    model: String,
    reasoning_effort: String,
    status: String,
    service_tier: String,
    incomplete_reason: String,
    error_code: String,
    error_message: String,
    error_param: String,
    sequence_number: Option<u64>,
    tok: u32,
    input: u32,
    cache: Option<u32>,
    output: u32,
    reasoning: u32,
    output_items: BTreeMap<String, u64>,
    output_items_human: String,
    calls: BTreeMap<String, u64>,
    calls_by_source: BTreeMap<String, BTreeMap<String, u64>>,
    calls_human: String,
}

impl ResponseFields {
    fn from_state(state: &ResponsesUpstreamState, sequence_number: Option<u64>) -> Self {
        let summary = state.effective_summary();
        let snapshot = state.snapshot.as_ref().map(|value| &value.projection);
        let error = state
            .effective_error()
            .map(|value| {
                (
                    value.code.clone(),
                    truncate_chars(&value.message, 120),
                    String::new(),
                )
            })
            .unwrap_or_default();

        let function_calls = string_count_map(&summary.function_calls);
        let mcp_calls = string_count_map(&summary.mcp_calls);

        Self {
            id: compact_tail(
                snapshot.map(|value| value.id.as_str()).unwrap_or_default(),
                8,
            ),
            model: snapshot
                .map(|value| value.model.clone())
                .unwrap_or_default(),
            reasoning_effort: snapshot
                .and_then(|value| value.reasoning.as_ref())
                .and_then(|value| value.effort)
                .map(|value| value.to_string())
                .unwrap_or_default(),
            status: snapshot
                .map(|value| value.status.to_string())
                .unwrap_or_default(),
            service_tier: snapshot
                .and_then(|value| value.service_tier)
                .map(|value| value.to_string())
                .unwrap_or_default(),
            incomplete_reason: snapshot
                .and_then(|value| value.incomplete_details.as_ref())
                .map(|value| value.reason.clone())
                .unwrap_or_default(),
            error_code: error.0,
            error_message: error.1,
            error_param: error.2,
            sequence_number,
            tok: snapshot
                .and_then(|value| value.usage.map(|usage| usage.total_tokens))
                .unwrap_or_default(),
            input: snapshot
                .and_then(|value| {
                    value.usage.map(|usage| {
                        usage
                            .input_tokens
                            .saturating_sub(usage.input_tokens_details.cached_tokens)
                    })
                })
                .unwrap_or_default(),
            cache: snapshot.and_then(|value| {
                value
                    .usage
                    .map(|usage| usage.input_tokens_details.cached_tokens)
            }),
            output: snapshot
                .and_then(|value| value.usage.map(|usage| usage.output_tokens))
                .unwrap_or_default(),
            reasoning: snapshot
                .and_then(|value| {
                    value
                        .usage
                        .map(|usage| usage.output_tokens_details.reasoning_tokens)
                })
                .unwrap_or_default(),
            output_items: string_count_map(&summary.output_items),
            output_items_human: compact_output_items_for_human(
                &summary.output_items,
                ResponseOutputItemKind::Message,
            ),
            calls: merge_count_maps([function_calls.clone(), mcp_calls.clone()]),
            calls_by_source: source_count_maps([("function", function_calls), ("mcp", mcp_calls)]),
            calls_human: join_call_maps([
                compact_tool_calls(&summary.function_calls),
                full_count_map(&summary.mcp_calls),
            ]),
        }
    }
}

impl ValuableJson for ResponseFields {
    fn to_json_value(&self) -> JsonValue {
        super::json_object([
            ("id", JsonValue::String(self.id.clone())),
            ("model", JsonValue::String(self.model.clone())),
            (
                "reasoning_effort",
                JsonValue::String(self.reasoning_effort.clone()),
            ),
            ("status", JsonValue::String(self.status.clone())),
            ("service_tier", JsonValue::String(self.service_tier.clone())),
            (
                "incomplete_reason",
                JsonValue::String(self.incomplete_reason.clone()),
            ),
            ("error_code", JsonValue::String(self.error_code.clone())),
            (
                "error_message",
                JsonValue::String(self.error_message.clone()),
            ),
            ("error_param", JsonValue::String(self.error_param.clone())),
            (
                "sequence_number",
                self.sequence_number
                    .map(JsonValue::from)
                    .unwrap_or(JsonValue::Null),
            ),
            ("tok", JsonValue::from(self.tok)),
            ("input", JsonValue::from(self.input)),
            (
                "cache",
                self.cache.map(JsonValue::from).unwrap_or(JsonValue::Null),
            ),
            ("output", JsonValue::from(self.output)),
            ("reasoning", JsonValue::from(self.reasoning)),
            (
                "output_items",
                serde_json::to_value(&self.output_items).unwrap_or(JsonValue::Null),
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
pub(crate) enum ResponsesLogRecord<'a> {
    Upstream(UpstreamLogRecord<'a>),
    StreamInfo {
        event: &'static str,
        snapshot: &'a ResponsesUpstreamStreamSnapshot,
    },
    StreamError {
        snapshot: &'a ResponsesUpstreamStreamSnapshot,
        error: &'a UpstreamResponseError,
    },
}

impl<'a> ResponsesLogRecord<'a> {
    pub(crate) fn from_event(event: &'a ResponsesUpstreamEvent) -> Self {
        match event {
            ResponsesUpstreamEvent::Headers { head } => {
                Self::Upstream(UpstreamLogRecord::HeadInfo { head })
            }
            ResponsesUpstreamEvent::Completed { snapshot } => Self::StreamInfo {
                event: "end",
                snapshot: snapshot.as_ref(),
            },
            ResponsesUpstreamEvent::Closed { snapshot } => Self::StreamInfo {
                event: "closed",
                snapshot: snapshot.as_ref(),
            },
            ResponsesUpstreamEvent::Error { snapshot, error } => Self::StreamError {
                snapshot: snapshot.as_ref(),
                error,
            },
        }
    }

    pub(crate) fn emit(self) {
        match self {
            Self::Upstream(record) => record.emit(),
            Self::StreamInfo { event, snapshot } => emit_stream_info(event, snapshot),
            Self::StreamError { snapshot, error } => emit_stream_error(snapshot, error),
        }
    }
}

fn response_fields_from_snapshot(snapshot: &ResponsesUpstreamStreamSnapshot) -> ResponseFields {
    ResponseFields::from_state(&snapshot.state, snapshot.state.sequence_number)
}

fn emit_stream_info(event: &str, snapshot: &ResponsesUpstreamStreamSnapshot) {
    let head = &snapshot.head;
    let response = response_fields_from_snapshot(snapshot);
    let rate_limit = snapshot.state.rate_limit;
    let codex_limits = snapshot.state.codex_limits;

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
            reasoning_effort = response.reasoning_effort,
            response_status = response.status,
            service_tier = response.service_tier,
            incomplete_reason = response.incomplete_reason,
            error_code = response.error_code,
            error_message = response.error_message,
            error_param = response.error_param,
            seq = response.sequence_number,
            tok = response.tok,
            input = response.input,
            cache = response.cache,
            output = response.output,
            reasoning = response.reasoning,
            output_items_human = response.output_items_human,
            calls_human = response.calls_human,
            rate_limit_limit_requests = rate_limit.limit_requests,
            rate_limit_limit_tokens = rate_limit.limit_tokens,
            rate_limit_remaining_requests = rate_limit.remaining_requests,
            rate_limit_remaining_tokens = rate_limit.remaining_tokens,
            rate_limit_reset_requests_ms = rate_limit
                .reset_requests
                .map(|value| value.as_millis() as u64),
            rate_limit_reset_tokens_ms = rate_limit
                .reset_tokens
                .map(|value| value.as_millis() as u64),
            codex_primary_used_percent = codex_limits.primary_used_percent,
            codex_primary_reset_after_secs = codex_limits.primary_reset_after_secs,
            codex_primary_window_minutes = codex_limits.primary_window_minutes,
            codex_secondary_used_percent = codex_limits.secondary_used_percent,
            codex_secondary_reset_after_secs = codex_limits.secondary_reset_after_secs,
            codex_secondary_window_minutes = codex_limits.secondary_window_minutes,
            codex_primary_over_secondary_percent = codex_limits.primary_over_secondary_percent,
        ),
        LogOutputFormat::Json => {
            let mut payload = response.to_json_value();
            rename_json_field(&mut payload, "id", "response_id");
            rename_json_field(&mut payload, "status", "response_status");
            rename_json_field(&mut payload, "sequence_number", "seq");
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
                    (
                        "sse",
                        JsonValue::Bool(
                            head.content_type.as_ref().is_some_and(ContentType::is_sse),
                        ),
                    ),
                    (
                        "rate_limit_limit_requests",
                        optional_u64(rate_limit.limit_requests),
                    ),
                    (
                        "rate_limit_limit_tokens",
                        optional_u64(rate_limit.limit_tokens),
                    ),
                    (
                        "rate_limit_remaining_requests",
                        optional_u64(rate_limit.remaining_requests),
                    ),
                    (
                        "rate_limit_remaining_tokens",
                        optional_u64(rate_limit.remaining_tokens),
                    ),
                    (
                        "rate_limit_reset_requests_ms",
                        optional_u64(
                            rate_limit
                                .reset_requests
                                .map(|value| value.as_millis() as u64),
                        ),
                    ),
                    (
                        "rate_limit_reset_tokens_ms",
                        optional_u64(
                            rate_limit
                                .reset_tokens
                                .map(|value| value.as_millis() as u64),
                        ),
                    ),
                    (
                        "codex_primary_used_percent",
                        optional_f64(codex_limits.primary_used_percent),
                    ),
                    (
                        "codex_primary_reset_after_secs",
                        optional_u64(codex_limits.primary_reset_after_secs),
                    ),
                    (
                        "codex_primary_window_minutes",
                        optional_u64(codex_limits.primary_window_minutes),
                    ),
                    (
                        "codex_secondary_used_percent",
                        optional_f64(codex_limits.secondary_used_percent),
                    ),
                    (
                        "codex_secondary_reset_after_secs",
                        optional_u64(codex_limits.secondary_reset_after_secs),
                    ),
                    (
                        "codex_secondary_window_minutes",
                        optional_u64(codex_limits.secondary_window_minutes),
                    ),
                    (
                        "codex_primary_over_secondary_percent",
                        optional_f64(codex_limits.primary_over_secondary_percent),
                    ),
                ],
            );
            emit_json_log("INFO", event, payload);
        }
    }
}

fn emit_stream_error(snapshot: &ResponsesUpstreamStreamSnapshot, error: &UpstreamResponseError) {
    emit_stream_error_with_diagnostic(snapshot, error, None);
}

pub(crate) fn emit_stream_error_with_diagnostic(
    snapshot: &ResponsesUpstreamStreamSnapshot,
    error: &UpstreamResponseError,
    diagnostic_path: Option<&str>,
) {
    let head = &snapshot.head;
    let response = response_fields_from_snapshot(snapshot);
    let rate_limit = snapshot.state.rate_limit;
    let codex_limits = snapshot.state.codex_limits;
    let response_error = match error {
        UpstreamResponseError::Protocol(error) => Some(error),
        _ => None,
    };
    let sequence_number = match error {
        UpstreamResponseError::UnfinishedTool { sequence_number } => *sequence_number,
        _ => response.sequence_number,
    };
    let error_code = if response_error
        .map(|value| value.code.as_str())
        .unwrap_or("")
        .is_empty()
    {
        response.error_code.clone()
    } else {
        response_error
            .map(|value| value.code.as_str())
            .unwrap_or("")
            .to_string()
    };
    let error_message = if response_error
        .map(|value| value.message.as_str())
        .unwrap_or("")
        .is_empty()
    {
        response.error_message.clone()
    } else {
        response_error
            .map(|value| value.message.as_str())
            .unwrap_or("")
            .to_string()
    };
    let error_param = response.error_param.clone();

    match active_log_format() {
        LogOutputFormat::Human => warn!(
            event = error_token(error),
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
            reasoning_effort = response.reasoning_effort,
            response_status = response.status,
            service_tier = response.service_tier,
            incomplete_reason = response.incomplete_reason,
            error_code = error_code,
            error_message = error_message,
            error_param = error_param,
            seq = sequence_number,
            timeout_ms = Option::<u64>::None,
            diagnostic_path = diagnostic_path.unwrap_or(""),
            tok = response.tok,
            input = response.input,
            cache = response.cache,
            output = response.output,
            reasoning = response.reasoning,
            output_items_human = response.output_items_human,
            calls_human = response.calls_human,
            rate_limit_limit_requests = rate_limit.limit_requests,
            rate_limit_limit_tokens = rate_limit.limit_tokens,
            rate_limit_remaining_requests = rate_limit.remaining_requests,
            rate_limit_remaining_tokens = rate_limit.remaining_tokens,
            rate_limit_reset_requests_ms = rate_limit
                .reset_requests
                .map(|value| value.as_millis() as u64),
            rate_limit_reset_tokens_ms = rate_limit
                .reset_tokens
                .map(|value| value.as_millis() as u64),
            codex_primary_used_percent = codex_limits.primary_used_percent,
            codex_primary_reset_after_secs = codex_limits.primary_reset_after_secs,
            codex_primary_window_minutes = codex_limits.primary_window_minutes,
            codex_secondary_used_percent = codex_limits.secondary_used_percent,
            codex_secondary_reset_after_secs = codex_limits.secondary_reset_after_secs,
            codex_secondary_window_minutes = codex_limits.secondary_window_minutes,
            codex_primary_over_secondary_percent = codex_limits.primary_over_secondary_percent,
            err = error_text(error),
        ),
        LogOutputFormat::Json => {
            let mut payload = response.to_json_value();
            rename_json_field(&mut payload, "id", "response_id");
            rename_json_field(&mut payload, "status", "response_status");
            rename_json_field(&mut payload, "sequence_number", "seq");
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
                    (
                        "sse",
                        JsonValue::Bool(
                            head.content_type.as_ref().is_some_and(ContentType::is_sse),
                        ),
                    ),
                    ("error_code", JsonValue::String(error_code)),
                    ("error_message", JsonValue::String(error_message)),
                    ("error_param", JsonValue::String(error_param)),
                    ("seq", optional_u64(sequence_number)),
                    ("timeout_ms", optional_u64(None)),
                    (
                        "diagnostic_path",
                        JsonValue::String(diagnostic_path.unwrap_or("").to_string()),
                    ),
                    (
                        "rate_limit_limit_requests",
                        optional_u64(rate_limit.limit_requests),
                    ),
                    (
                        "rate_limit_limit_tokens",
                        optional_u64(rate_limit.limit_tokens),
                    ),
                    (
                        "rate_limit_remaining_requests",
                        optional_u64(rate_limit.remaining_requests),
                    ),
                    (
                        "rate_limit_remaining_tokens",
                        optional_u64(rate_limit.remaining_tokens),
                    ),
                    (
                        "rate_limit_reset_requests_ms",
                        optional_u64(
                            rate_limit
                                .reset_requests
                                .map(|value| value.as_millis() as u64),
                        ),
                    ),
                    (
                        "rate_limit_reset_tokens_ms",
                        optional_u64(
                            rate_limit
                                .reset_tokens
                                .map(|value| value.as_millis() as u64),
                        ),
                    ),
                    (
                        "codex_primary_used_percent",
                        optional_f64(codex_limits.primary_used_percent),
                    ),
                    (
                        "codex_primary_reset_after_secs",
                        optional_u64(codex_limits.primary_reset_after_secs),
                    ),
                    (
                        "codex_primary_window_minutes",
                        optional_u64(codex_limits.primary_window_minutes),
                    ),
                    (
                        "codex_secondary_used_percent",
                        optional_f64(codex_limits.secondary_used_percent),
                    ),
                    (
                        "codex_secondary_reset_after_secs",
                        optional_u64(codex_limits.secondary_reset_after_secs),
                    ),
                    (
                        "codex_secondary_window_minutes",
                        optional_u64(codex_limits.secondary_window_minutes),
                    ),
                    (
                        "codex_primary_over_secondary_percent",
                        optional_f64(codex_limits.primary_over_secondary_percent),
                    ),
                    ("err", JsonValue::String(error_text(error).to_string())),
                ],
            );
            emit_json_log("WARN", error_token(error), payload);
        }
    }
}

#[cfg(test)]
#[path = "openai_responses_tests.rs"]
mod tests;
