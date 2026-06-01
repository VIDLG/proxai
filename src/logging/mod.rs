mod anthropic_messages;
mod human;
mod openai_chat_completions;
mod openai_responses;
mod record;
mod request_hints;
mod tool_alias;
mod upstream;

use axum::http::{Method, Uri};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::io::Write;
use std::sync::OnceLock;
use tracing::info;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

use crate::config::{LogLevel, LogOutputFormat};
use crate::provider::ForwardedRequestView;
pub(crate) use anthropic_messages::AnthropicLogRecord;
pub(crate) use openai_chat_completions::ChatLogRecord;
pub(crate) use openai_responses::{
    emit_stream_error_with_diagnostic as emit_responses_stream_error_with_diagnostic,
    ResponsesLogRecord,
};
use record::{ForwardFields, ValuableJson};
pub(crate) use tool_alias::compact_tool_call_name;
pub use tool_alias::TOOL_NAME_ALIASES;
pub(crate) use upstream::UpstreamLogRecord;

pub use human::DurationThresholds;

use human::HumanLayer;

static ACTIVE_LOG_FORMAT: OnceLock<LogOutputFormat> = OnceLock::new();

pub fn init(
    level: LogLevel,
    format: LogOutputFormat,
    use_color: bool,
    duration_thresholds: DurationThresholds,
) {
    let _ = ACTIVE_LOG_FORMAT.set(format);
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level.to_string()));

    match format {
        LogOutputFormat::Human => tracing_subscriber::registry()
            .with(filter)
            .with(HumanLayer::new(duration_thresholds, use_color))
            .init(),
        LogOutputFormat::Json => tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_ansi(false)
                    .json(),
            )
            .init(),
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RequestBodySizes {
    pub(crate) inbound: u64,
    pub(crate) forwarded: u64,
}

/// Request-level logging payload for the initial forward event.
pub(crate) struct ForwardedRequestEvent<'a> {
    pub(crate) request_id: u64,
    pub(crate) method: Method,
    pub(crate) uri: Uri,
    pub(crate) request_sizes: RequestBodySizes,
    pub(crate) forwarded_request: ForwardedRequestView<'a>,
    pub(crate) capture: bool,
}

impl RequestBodySizes {
    pub(crate) fn delta(self) -> i128 {
        self.forwarded as i128 - self.inbound as i128
    }
}

impl ForwardedRequestEvent<'_> {
    pub(crate) fn emit(&self) {
        let fields = ForwardFields::from(self);
        match active_log_format() {
            LogOutputFormat::Human => info!(
                event = "fwd",
                id = fields.request_id,
                method = fields.method,
                path = fields.path,
                forwarded_request_bytes = fields.forwarded_request_bytes,
                inbound_request_bytes = fields.inbound_request_bytes,
                request_delta_bytes = fields.request_delta_bytes,
                model = fields.model,
                reasoning_effort = fields.reasoning_effort,
                stream = fields.stream,
                max_output_tokens = fields.max_output_tokens,
                request_hints = fields.request_hints,
                capture = fields.capture,
            ),
            LogOutputFormat::Json => emit_json_log("INFO", "fwd", fields.to_json_value()),
        }
    }
}

pub(crate) fn active_log_format() -> LogOutputFormat {
    ACTIVE_LOG_FORMAT
        .get()
        .copied()
        .unwrap_or(LogOutputFormat::Human)
}

pub(crate) fn emit_json_log(level: &str, event: &str, payload: JsonValue) {
    let mut object = match payload {
        JsonValue::Object(map) => map,
        other => {
            let mut map = JsonMap::new();
            map.insert("record".to_string(), other);
            map
        }
    };
    object.insert("level".to_string(), JsonValue::String(level.to_string()));
    object.insert("event".to_string(), JsonValue::String(event.to_string()));
    object.insert(
        "ts".to_string(),
        JsonValue::String(chrono::Local::now().to_rfc3339()),
    );

    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{}", JsonValue::Object(object));
}

pub(crate) fn json_object<const N: usize>(entries: [(&str, JsonValue); N]) -> JsonValue {
    let mut map = JsonMap::new();
    for (key, value) in entries {
        map.insert(key.to_string(), value);
    }
    JsonValue::Object(map)
}

pub(crate) fn extend_json_object<const N: usize>(
    value: &mut JsonValue,
    entries: [(&str, JsonValue); N],
) {
    if let JsonValue::Object(map) = value {
        for (key, value) in entries {
            map.insert(key.to_string(), value);
        }
    }
}

pub(crate) fn rename_json_field(value: &mut JsonValue, from: &str, to: &str) {
    if let JsonValue::Object(map) = value {
        if let Some(value) = map.remove(from) {
            map.insert(to.to_string(), value);
        }
    }
}

pub(crate) fn optional_u64(value: Option<u64>) -> JsonValue {
    value.map(JsonValue::from).unwrap_or(JsonValue::Null)
}

pub(crate) fn optional_f64(value: Option<f64>) -> JsonValue {
    value.map(JsonValue::from).unwrap_or(JsonValue::Null)
}
