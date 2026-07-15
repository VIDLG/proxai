use std::path::Path;

use serde_json::{Value as JsonValue, json};
use tracing::warn;

use crate::config::LogOutputFormat;
use crate::observe::point::{RequestTranslationFailure, StreamingTranslationFailure};
use crate::request::RequestId;

use super::{
    active_log_format, compact_provider_protocol, compact_request_protocol, emit_json_log,
    render_translation, render_translation_alias,
};

pub(super) fn emit_request_translation_failure(
    request_id: RequestId,
    point: &RequestTranslationFailure<'_>,
    diagnostic_path: Option<&Path>,
) {
    let stream = point
        .normalized_payload
        .get("stream")
        .and_then(JsonValue::as_bool);
    let max_output_tokens = point
        .normalized_payload
        .get("max_output_tokens")
        .and_then(JsonValue::as_u64);
    let reasoning_effort = point
        .normalized_payload
        .pointer("/reasoning/effort")
        .and_then(JsonValue::as_str)
        .unwrap_or("");
    let tools = point
        .normalized_payload
        .get("tools")
        .and_then(JsonValue::as_array)
        .map(Vec::len);
    let request_hints = tools
        .map(|count| format!("tools[{count}]"))
        .unwrap_or_default();
    let json_path = point
        .error
        .as_json_payload_error()
        .map(|error| error.path().as_str())
        .unwrap_or_default();
    let diagnostic_path = diagnostic_path
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let request_protocol = point.request_protocol.to_string();
    let provider_protocol = point.provider_protocol.to_string();
    let translation = render_translation(point.request_protocol, point.provider_protocol);
    let request_protocol_alias = compact_request_protocol(point.request_protocol);
    let provider_protocol_alias = compact_provider_protocol(point.provider_protocol);
    let translation_alias =
        render_translation_alias(point.request_protocol, point.provider_protocol);

    match active_log_format() {
        LogOutputFormat::Human => warn!(
            event = "fwd-error",
            request_id = request_id.as_u64(),
            method = %point.method,
            path = point.uri.path(),
            inbound_request_bytes = point.inbound_request_bytes,
            request_protocol,
            provider = point.provider,
            route_name = point.route_name.unwrap_or(""),
            provider_protocol,
            translation,
            request_protocol_alias,
            translation_alias,
            provider_protocol_alias,
            model = point.model,
            reasoning_effort,
            stream = ?stream,
            max_output_tokens = ?max_output_tokens,
            request_hints,
            json_path,
            diagnostic_path,
            err = %point.error,
            "request translation failed before forwarding"
        ),
        LogOutputFormat::Json => emit_json_log(
            "WARN",
            "fwd-error",
            json!({
                "request_id": request_id,
                "method": point.method.as_str(),
                "path": point.uri.path(),
                "inbound_request_bytes": point.inbound_request_bytes,
                "request_protocol": request_protocol,
                "provider": point.provider,
                "route_name": point.route_name,
                "provider_protocol": provider_protocol,
                "translation": translation,
                "request_protocol_alias": request_protocol_alias,
                "translation_alias": translation_alias,
                "provider_protocol_alias": provider_protocol_alias,
                "model": point.model,
                "reasoning_effort": reasoning_effort,
                "stream": stream,
                "max_output_tokens": max_output_tokens,
                "request_hints": request_hints,
                "json_path": json_path,
                "diagnostic_path": diagnostic_path,
                "error": point.error.to_string(),
            }),
        ),
    }
}

pub(super) fn emit_streaming_translation_failure(
    request_id: RequestId,
    point: &StreamingTranslationFailure<'_>,
    diagnostic_path: Option<&Path>,
) {
    let request_protocol = point.request_protocol.to_string();
    let provider_protocol = point.provider_protocol.to_string();
    let translation = render_translation(point.request_protocol, point.provider_protocol);
    let request_protocol_alias = compact_request_protocol(point.request_protocol);
    let provider_protocol_alias = compact_provider_protocol(point.provider_protocol);
    let translation_alias =
        render_translation_alias(point.request_protocol, point.provider_protocol);
    let diagnostic_path = diagnostic_path
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let json_path = point
        .failure
        .error
        .as_json_payload_error()
        .map(|error| error.path().as_str())
        .unwrap_or_default();
    let error = point.failure.to_string();
    let upstream_event_type = point
        .failure
        .upstream_event
        .as_ref()
        .map(|event| event.event_type.as_str())
        .unwrap_or("");
    let stream_end = point
        .failure
        .end
        .map(|end| end.to_string())
        .unwrap_or_default();

    match active_log_format() {
        LogOutputFormat::Human => warn!(
            event = "stream-error",
            kind = "translation",
            request_id = request_id.as_u64(),
            method = %point.method,
            path = point.uri.path(),
            request_protocol,
            provider_protocol,
            translation,
            request_protocol_alias,
            translation_alias,
            provider_protocol_alias,
            stage = point.failure.stage.as_ref(),
            upstream_event_type,
            stream_end,
            diagnostic_path,
            json_path,
            err = %error,
            "stream translation failed after forwarding"
        ),
        LogOutputFormat::Json => emit_json_log(
            "WARN",
            "stream-error",
            json!({
                "request_id": request_id,
                "method": point.method.as_str(),
                "path": point.uri.path(),
                "request_protocol": request_protocol,
                "provider_protocol": provider_protocol,
                "translation": translation,
                "request_protocol_alias": request_protocol_alias,
                "translation_alias": translation_alias,
                "provider_protocol_alias": provider_protocol_alias,
                "stage": point.failure.stage.as_ref(),
                "upstream_event_type": upstream_event_type,
                "stream_end": stream_end,
                "diagnostic_path": diagnostic_path,
                "json_path": json_path,
                "error": error,
            }),
        ),
    }
}
