use serde_json::Value as JsonValue;
use tracing::{info, warn};

use crate::config::LogOutputFormat;
use crate::error::{UpstreamError, UpstreamResponseError};
use crate::http_model::UpstreamResponseHead;
use crate::upstream::UpstreamStreamError;

use super::{active_log_format, emit_json_log, json_object};

#[derive(Clone, Copy)]
pub(crate) enum UpstreamLogRecord<'a> {
    HeadInfo {
        head: &'a UpstreamResponseHead,
    },
    HeadError {
        head: &'a UpstreamResponseHead,
        error: &'a UpstreamError,
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

pub(crate) fn stream_error_token(error: &UpstreamStreamError) -> &'static str {
    match error {
        UpstreamStreamError::Stream { .. } => "stream-error",
        UpstreamStreamError::UnfinishedTool { .. } => "unfinished-tool",
    }
}

pub(crate) fn stream_error_text(error: &UpstreamStreamError) -> String {
    match error {
        UpstreamStreamError::Stream { message } => message.clone(),
        UpstreamStreamError::UnfinishedTool { .. } => error.to_string(),
    }
}

pub(crate) fn error_text(error: &UpstreamError) -> String {
    match error {
        UpstreamError::RequestSend(error) => error.to_string(),
        UpstreamError::ErrorStatus { parsed, .. } => response_error_text(parsed),
        UpstreamError::ResponseBodyRead { source, .. } => source.to_string(),
    }
}

fn response_error_text(error: &UpstreamResponseError) -> String {
    match error {
        UpstreamResponseError::Upstream { message, .. } => message.clone(),
        UpstreamResponseError::EmptyBody
        | UpstreamResponseError::NonJsonBody { .. }
        | UpstreamResponseError::UnknownBodyShape { .. } => error.to_string(),
    }
}

fn upstream_error_code(error: &UpstreamError) -> Option<&str> {
    match error {
        UpstreamError::ErrorStatus { parsed, .. } => parsed.upstream_code(),
        UpstreamError::RequestSend(_) | UpstreamError::ResponseBodyRead { .. } => None,
    }
}

fn upstream_error_message(error: &UpstreamError) -> Option<&str> {
    match error {
        UpstreamError::ErrorStatus { parsed, .. } => parsed.upstream_message(),
        UpstreamError::RequestSend(_) | UpstreamError::ResponseBodyRead { .. } => None,
    }
}

fn emit_head_info(head: &UpstreamResponseHead) {
    if matches!(active_log_format(), LogOutputFormat::Human) && is_default_success_sse_head(head) {
        return;
    }

    let content_type = head.content_type_text();
    let transfer_encoding = head.transfer_encoding_text();
    let content_length = head.content_length().unwrap_or_default();

    match active_log_format() {
        LogOutputFormat::Human => info!(
            event = "hdr",
            status = head.status.as_u16(),
            down = content_length,
            ttfb_ms = head.ttfb.as_millis() as u64,
            ct = content_type,
            te = transfer_encoding,
            sse = head.is_sse(),
        ),
        LogOutputFormat::Json => emit_json_log(
            "INFO",
            "hdr",
            json_object([
                ("status", JsonValue::from(head.status.as_u16())),
                ("down", JsonValue::from(content_length)),
                ("ttfb_ms", JsonValue::from(head.ttfb.as_millis() as u64)),
                ("ct", JsonValue::String(content_type)),
                ("te", JsonValue::String(transfer_encoding.to_string())),
                ("sse", JsonValue::Bool(head.is_sse())),
            ]),
        ),
    }
}

fn is_default_success_sse_head(head: &UpstreamResponseHead) -> bool {
    head.status.is_success()
        && head.is_sse()
        && head
            .transfer_encoding()
            .is_some_and(|value| value.eq_ignore_ascii_case("chunked"))
}

fn emit_head_error(head: &UpstreamResponseHead, error: &UpstreamError) {
    let error_code = upstream_error_code(error).unwrap_or("");
    let error_message = upstream_error_message(error).unwrap_or("");
    let content_type = head.content_type_text();
    let transfer_encoding = head.transfer_encoding_text();
    let content_length = head.content_length().unwrap_or_default();

    match active_log_format() {
        LogOutputFormat::Human => warn!(
            event = "hdr-error",
            status = head.status.as_u16(),
            down = content_length,
            ttfb_ms = head.ttfb.as_millis() as u64,
            ct = content_type,
            te = transfer_encoding,
            sse = head.is_sse(),
            error_code,
            error_message,
            error_param = "",
            err = error_text(error),
        ),
        LogOutputFormat::Json => emit_json_log(
            "WARN",
            "hdr-error",
            json_object([
                ("status", JsonValue::from(head.status.as_u16())),
                ("down", JsonValue::from(content_length)),
                ("ttfb_ms", JsonValue::from(head.ttfb.as_millis() as u64)),
                ("ct", JsonValue::String(content_type)),
                ("te", JsonValue::String(transfer_encoding.to_string())),
                ("sse", JsonValue::Bool(head.is_sse())),
                ("error_code", JsonValue::String(error_code.to_string())),
                (
                    "error_message",
                    JsonValue::String(error_message.to_string()),
                ),
                ("error_param", JsonValue::String(String::new())),
                ("err", JsonValue::String(error_text(error))),
            ]),
        ),
    }
}
