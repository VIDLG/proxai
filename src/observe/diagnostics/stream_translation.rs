use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::observe::point::StreamingTranslationFailure;
use crate::paths;
use crate::request::RequestId;

const KIND: &str = "stream_translation_failure";
const PHASE: &str = "outbound_response";

pub(super) fn write_streaming_translation_failure(
    request_id: RequestId,
    point: &StreamingTranslationFailure<'_>,
) -> Option<PathBuf> {
    let diagnostics_dir = paths::ensure_app_paths().ok()?.diagnostics_dir;
    write_streaming_translation_failure_to_dir(request_id, point, &diagnostics_dir)
}

fn write_streaming_translation_failure_to_dir(
    request_id: RequestId,
    point: &StreamingTranslationFailure<'_>,
    diagnostics_dir: &Path,
) -> Option<PathBuf> {
    fs::create_dir_all(diagnostics_dir).ok()?;

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let id = format!("{timestamp}-{:06}-{KIND}", request_id.as_u64());
    let bundle_dir = diagnostics_dir.join(&id);
    fs::create_dir_all(&bundle_dir).ok()?;

    let upstream_sse_event = match point.failure.upstream_event.as_ref() {
        Some(event) => {
            let filename = "upstream_sse_event.sse";
            let mut frame = String::new();
            if event.event_type != crate::sse::SseEvent::DEFAULT_EVENT_TYPE {
                frame.push_str("event: ");
                frame.push_str(&event.event_type);
                frame.push('\n');
            }
            frame.push_str("data: ");
            frame.push_str(&event.data);
            frame.push_str("\n\n");
            fs::write(bundle_dir.join(filename), frame).ok()?;
            Some(filename)
        }
        None => None,
    };

    let record = DiagnosticRecord {
        id,
        created_at: Utc::now().to_rfc3339(),
        request_id: request_id.as_u64(),
        kind: KIND,
        phase: PHASE,
        summary: "Failed to translate a provider SSE stream to the inbound protocol",
        request: DiagnosticRequest {
            method: point.method.as_str().to_string(),
            path: point.uri.path().to_string(),
            request_protocol: point.request_protocol.to_string(),
            provider_protocol: point.provider_protocol.to_string(),
        },
        failure: DiagnosticFailure {
            stage: point.failure.stage.as_ref().to_string(),
            error: &point.failure.error,
            stream_end: point.failure.end.map(|end| end.to_string()),
            upstream_event_type: point
                .failure
                .upstream_event
                .as_ref()
                .map(|event| event.event_type.as_str()),
        },
        artifacts: DiagnosticArtifacts { upstream_sse_event },
    };
    fs::write(
        bundle_dir.join("record.json"),
        serde_json::to_vec_pretty(&record).ok()?,
    )
    .ok()?;

    super::trim_old_records(diagnostics_dir);
    Some(bundle_dir)
}

#[derive(Serialize)]
struct DiagnosticRecord<'a> {
    id: String,
    created_at: String,
    request_id: u64,
    kind: &'static str,
    phase: &'static str,
    summary: &'static str,
    request: DiagnosticRequest,
    failure: DiagnosticFailure<'a>,
    artifacts: DiagnosticArtifacts<'a>,
}

#[derive(Serialize)]
struct DiagnosticRequest {
    method: String,
    path: String,
    request_protocol: String,
    provider_protocol: String,
}

#[derive(Serialize)]
struct DiagnosticFailure<'a> {
    stage: String,
    error: &'a str,
    stream_end: Option<String>,
    upstream_event_type: Option<&'a str>,
}

#[derive(Serialize)]
struct DiagnosticArtifacts<'a> {
    upstream_sse_event: Option<&'a str>,
}

#[cfg(test)]
#[path = "stream_translation_tests.rs"]
mod tests;
