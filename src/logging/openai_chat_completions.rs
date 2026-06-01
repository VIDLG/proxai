use serde_json::Value as JsonValue;
use tracing::{info, warn};
use valuable::Valuable;
use valuable_serde::Serializable;

use crate::config::LogOutputFormat;
use crate::formatting::{compact_tail, format_count_map};
use crate::provider::openai::chat_completions::ChatUpstreamStreamSnapshot;

use super::record::ValuableJson;
use super::{
    active_log_format, emit_json_log, extend_json_object, rename_json_field, UpstreamLogRecord,
};

#[derive(Debug, Clone, Default, Valuable)]
struct ChatResponseFields {
    id: String,
    model: String,
    service_tier: String,
    tok: u32,
    input: u32,
    cache: Option<u32>,
    output: u32,
    reasoning: u32,
    output_items: String,
    finish_reasons: String,
    tool_calls: String,
    custom_tool_calls: String,
}

impl From<&ChatUpstreamStreamSnapshot> for ChatResponseFields {
    fn from(snapshot: &ChatUpstreamStreamSnapshot) -> Self {
        let projection = snapshot.state.observed.latest.as_ref();
        let usage = projection.and_then(|projection| projection.usage());
        let summary = snapshot.state.observed.effective_summary();

        Self {
            id: compact_tail(
                projection
                    .map(|projection| projection.id())
                    .unwrap_or_default(),
                8,
            ),
            model: projection
                .map(|projection| projection.model().to_string())
                .unwrap_or_default(),
            service_tier: projection
                .and_then(|projection| projection.service_tier())
                .map(|value| value.to_string())
                .unwrap_or_default(),
            tok: usage.map(|usage| usage.total_tokens).unwrap_or_default(),
            input: usage
                .map(|usage| {
                    usage.prompt_tokens.saturating_sub(
                        usage
                            .prompt_tokens_details
                            .and_then(|details| details.cached_tokens)
                            .unwrap_or_default(),
                    )
                })
                .unwrap_or_default(),
            cache: usage.and_then(|usage| {
                usage
                    .prompt_tokens_details
                    .and_then(|details| details.cached_tokens)
            }),
            output: usage
                .map(|usage| usage.completion_tokens)
                .unwrap_or_default(),
            reasoning: usage
                .and_then(|usage| {
                    usage
                        .completion_tokens_details
                        .and_then(|details| details.reasoning_tokens)
                })
                .unwrap_or_default(),
            output_items: format_count_map(&summary.output_items),
            finish_reasons: format_count_map(&summary.finish_reasons),
            tool_calls: summary
                .tool_calls
                .iter()
                .map(|(key, value)| format!("{}:{value}", super::compact_tool_call_name(key)))
                .collect::<Vec<_>>()
                .join(" "),
            custom_tool_calls: summary
                .custom_tool_calls
                .iter()
                .map(|(key, value)| format!("{}:{value}", super::compact_tool_call_name(key)))
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

impl ValuableJson for ChatResponseFields {
    fn to_json_value(&self) -> JsonValue {
        serde_json::to_value(Serializable::new(self)).unwrap_or(JsonValue::Null)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ChatLogRecord<'a> {
    Upstream(UpstreamLogRecord<'a>),
    Completed {
        snapshot: &'a ChatUpstreamStreamSnapshot,
    },
    Closed {
        snapshot: &'a ChatUpstreamStreamSnapshot,
    },
    StreamError {
        snapshot: &'a ChatUpstreamStreamSnapshot,
        message: &'a str,
    },
}

impl ChatLogRecord<'_> {
    pub(crate) fn emit(self) {
        match self {
            Self::Upstream(record) => record.emit(),
            Self::Completed { snapshot } => emit_chat_stream_info("end", snapshot),
            Self::Closed { snapshot } => emit_chat_stream_info("closed", snapshot),
            Self::StreamError { snapshot, message } => emit_chat_stream_error(snapshot, message),
        }
    }
}

fn chat_response_fields_from_snapshot(snapshot: &ChatUpstreamStreamSnapshot) -> ChatResponseFields {
    ChatResponseFields::from(snapshot)
}

fn emit_chat_stream_info(event: &str, snapshot: &ChatUpstreamStreamSnapshot) {
    let head = &snapshot.head;
    let response = chat_response_fields_from_snapshot(snapshot);

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
            tok = response.tok,
            input = response.input,
            cache = response.cache,
            output = response.output,
            reasoning = response.reasoning,
            output_items = response.output_items,
            finish_reasons = response.finish_reasons,
            tool_calls = response.tool_calls,
            custom_tool_calls = response.custom_tool_calls,
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

fn emit_chat_stream_error(snapshot: &ChatUpstreamStreamSnapshot, message: &str) {
    let head = &snapshot.head;
    let response = chat_response_fields_from_snapshot(snapshot);

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
            tok = response.tok,
            input = response.input,
            cache = response.cache,
            output = response.output,
            reasoning = response.reasoning,
            output_items = response.output_items,
            finish_reasons = response.finish_reasons,
            tool_calls = response.tool_calls,
            custom_tool_calls = response.custom_tool_calls,
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
