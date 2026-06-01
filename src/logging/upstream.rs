use serde_json::Value as JsonValue;
use tracing::{info, warn};

use crate::config::LogOutputFormat;
use crate::provider::UpstreamResponseError;
use crate::upstream::{ContentType, UpstreamResponseHead};

use super::{active_log_format, emit_json_log, json_object};

#[derive(Clone, Copy)]
pub(crate) enum UpstreamLogRecord<'a> {
    HeadInfo {
        head: &'a UpstreamResponseHead,
    },
    HeadError {
        head: &'a UpstreamResponseHead,
        error: &'a UpstreamResponseError,
    },
}

impl UpstreamLogRecord<'_> {
    pub(crate) fn emit(self) {
        match self {
            Self::HeadInfo { head } => emit_head_info(head),
            Self::HeadError { head, error } => emit_head_error(head, error),
        }
    }
}

pub(crate) fn error_token(error: &UpstreamResponseError) -> &'static str {
    match error {
        UpstreamResponseError::Protocol(_)
        | UpstreamResponseError::Proxy { .. }
        | UpstreamResponseError::Stream { .. } => "stream-error",
        UpstreamResponseError::UnfinishedTool { .. } => "unfinished-tool",
    }
}

pub(crate) fn error_text(error: &UpstreamResponseError) -> &str {
    match error {
        UpstreamResponseError::Protocol(error) => error.message.as_str(),
        UpstreamResponseError::Proxy { message } | UpstreamResponseError::Stream { message } => {
            message.as_str()
        }
        UpstreamResponseError::UnfinishedTool { .. } => {
            "upstream stream ended with unfinished tool arguments"
        }
    }
}

fn emit_head_info(head: &UpstreamResponseHead) {
    match active_log_format() {
        LogOutputFormat::Human => info!(
            event = "hdr",
            status = head.status.as_u16(),
            down = head.content_length.unwrap_or_default(),
            ttfb_ms = head.ttfb.as_millis() as u64,
            ct = head
                .content_type
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            te = head
                .transfer_encoding
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            sse = head.content_type.as_ref().is_some_and(ContentType::is_sse),
        ),
        LogOutputFormat::Json => emit_json_log(
            "INFO",
            "hdr",
            json_object([
                ("status", JsonValue::from(head.status.as_u16())),
                (
                    "down",
                    JsonValue::from(head.content_length.unwrap_or_default()),
                ),
                ("ttfb_ms", JsonValue::from(head.ttfb.as_millis() as u64)),
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
                    "te",
                    JsonValue::String(
                        head.transfer_encoding
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default(),
                    ),
                ),
                (
                    "sse",
                    JsonValue::Bool(head.content_type.as_ref().is_some_and(ContentType::is_sse)),
                ),
            ]),
        ),
    }
}

fn emit_head_error(head: &UpstreamResponseHead, error: &UpstreamResponseError) {
    let response_error = match error {
        UpstreamResponseError::Protocol(error) => Some(error),
        _ => None,
    };

    match active_log_format() {
        LogOutputFormat::Human => warn!(
            event = "hdr-error",
            status = head.status.as_u16(),
            down = head.content_length.unwrap_or_default(),
            ttfb_ms = head.ttfb.as_millis() as u64,
            ct = head
                .content_type
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            te = head
                .transfer_encoding
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            sse = head.content_type.as_ref().is_some_and(ContentType::is_sse),
            error_code = response_error
                .map(|value| value.code.as_str())
                .unwrap_or(""),
            error_message = response_error
                .map(|value| value.message.as_str())
                .unwrap_or(""),
            error_param = "",
            err = error_text(error),
        ),
        LogOutputFormat::Json => emit_json_log(
            "WARN",
            "hdr-error",
            json_object([
                ("status", JsonValue::from(head.status.as_u16())),
                (
                    "down",
                    JsonValue::from(head.content_length.unwrap_or_default()),
                ),
                ("ttfb_ms", JsonValue::from(head.ttfb.as_millis() as u64)),
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
                    "te",
                    JsonValue::String(
                        head.transfer_encoding
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default(),
                    ),
                ),
                (
                    "sse",
                    JsonValue::Bool(head.content_type.as_ref().is_some_and(ContentType::is_sse)),
                ),
                (
                    "error_code",
                    JsonValue::String(
                        response_error
                            .map(|value| value.code.as_str())
                            .unwrap_or("")
                            .to_string(),
                    ),
                ),
                (
                    "error_message",
                    JsonValue::String(
                        response_error
                            .map(|value| value.message.as_str())
                            .unwrap_or("")
                            .to_string(),
                    ),
                ),
                ("error_param", JsonValue::String(String::new())),
                ("err", JsonValue::String(error_text(error).to_string())),
            ]),
        ),
    }
}
