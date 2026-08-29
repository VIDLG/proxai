use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths;
use crate::protocol::ErrorObject;
use crate::protocol::anthropic::messages::StopReason;
use crate::provider::anthropic_messages::AnthropicUpstreamResponseSnapshot;
use crate::provider::openai::responses::{ResponseSummary, ResponsesUpstreamStreamSnapshot};
use crate::request::RequestId;
use crate::sse::SseEvent;

const KIND: &str = "provider_semantic_failure";

pub(super) fn write_openai_responses_error(
    request_id: RequestId,
    snapshot: &ResponsesUpstreamStreamSnapshot,
) -> Option<String> {
    snapshot.state.effective_error()?;
    let diagnostics_dir = paths::ensure_app_paths().ok()?.diagnostics_dir;
    write_openai_responses_error_to_dir(request_id, snapshot, &diagnostics_dir)
        .map(|path| path.display().to_string())
}

pub(super) fn write_anthropic_context_window_exceeded(
    request_id: RequestId,
    snapshot: &AnthropicUpstreamResponseSnapshot,
) -> Option<String> {
    if snapshot.state.stop_reason() != Some(StopReason::ModelContextWindowExceeded) {
        return None;
    }

    let diagnostics_dir = paths::ensure_app_paths().ok()?.diagnostics_dir;
    write_anthropic_context_window_exceeded_to_dir(request_id, snapshot, &diagnostics_dir)
        .map(|path| path.display().to_string())
}

fn write_openai_responses_error_to_dir(
    request_id: RequestId,
    snapshot: &ResponsesUpstreamStreamSnapshot,
    diagnostics_dir: &Path,
) -> Option<PathBuf> {
    let error = snapshot.state.effective_error()?.clone();
    let (id, bundle_dir) = create_bundle(request_id, diagnostics_dir)?;
    let terminal_event = snapshot
        .state
        .terminal_error_event
        .as_ref()
        .and_then(|event| write_terminal_event(&bundle_dir, event));
    let response_snapshot = snapshot.state.latest_snapshot.as_ref();
    let projection = response_snapshot.map(|snapshot| &snapshot.projection);
    let record = DiagnosticRecord {
        id,
        created_at: Utc::now().to_rfc3339(),
        request_id: request_id.as_u64(),
        kind: KIND,
        provider_protocol: "openai_responses",
        summary: "OpenAI Responses provider completed the stream with a semantic error",
        upstream: DiagnosticUpstream::from_openai_responses(snapshot),
        response: OpenaiResponsesDiagnosticResponse {
            id: projection.map(|response| response.id.clone()),
            model: projection.map(|response| response.model.clone()),
            status: projection.map(|response| response.status.to_string()),
            sequence_number: snapshot.state.sequence_number,
            snapshot_kind: response_snapshot.map(|snapshot| format!("{:?}", snapshot.kind)),
            error,
            summary: snapshot.state.effective_summary(),
        },
        artifacts: DiagnosticArtifacts { terminal_event },
    };
    fs::write(
        bundle_dir.join("record.json"),
        serde_json::to_vec_pretty(&record).ok()?,
    )
    .ok()?;

    super::trim_old_records(diagnostics_dir);
    Some(bundle_dir)
}

fn write_anthropic_context_window_exceeded_to_dir(
    request_id: RequestId,
    snapshot: &AnthropicUpstreamResponseSnapshot,
    diagnostics_dir: &Path,
) -> Option<PathBuf> {
    let (id, bundle_dir) = create_bundle(request_id, diagnostics_dir)?;
    let terminal_event = snapshot
        .state
        .terminal_event()
        .as_ref()
        .and_then(|event| write_terminal_event(&bundle_dir, event));

    let record = DiagnosticRecord {
        id,
        created_at: Utc::now().to_rfc3339(),
        request_id: request_id.as_u64(),
        kind: KIND,
        provider_protocol: "anthropic_messages",
        summary: "Anthropic provider exhausted the model context window before generation",
        upstream: DiagnosticUpstream {
            status: snapshot.head.status.as_u16(),
            content_type: snapshot.head.content_type_text(),
            ttfb_ms: snapshot.head.ttfb.as_millis() as u64,
            duration_ms: snapshot.metrics.duration_ms(),
            chunks: snapshot.metrics.chunks,
            bytes: snapshot.metrics.bytes,
        },
        response: AnthropicDiagnosticResponse {
            model: snapshot.state.model().clone(),
            stop_reason: snapshot
                .state
                .stop_reason()
                .map(|reason| reason.to_string()),
            input_tokens: snapshot.state.input_tokens(),
            output_tokens: snapshot.state.output_tokens(),
            output_items: snapshot
                .state
                .summary
                .output_items
                .iter()
                .map(|(kind, count)| (kind.to_string(), *count))
                .collect(),
        },
        artifacts: DiagnosticArtifacts { terminal_event },
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
struct DiagnosticRecord<R> {
    id: String,
    created_at: String,
    request_id: u64,
    kind: &'static str,
    provider_protocol: &'static str,
    summary: &'static str,
    upstream: DiagnosticUpstream,
    response: R,
    artifacts: DiagnosticArtifacts,
}

fn create_bundle(request_id: RequestId, diagnostics_dir: &Path) -> Option<(String, PathBuf)> {
    fs::create_dir_all(diagnostics_dir).ok()?;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let id = format!("{timestamp}-{:06}-{KIND}", request_id.as_u64());
    let bundle_dir = diagnostics_dir.join(&id);
    fs::create_dir_all(&bundle_dir).ok()?;
    Some((id, bundle_dir))
}

fn write_terminal_event(bundle_dir: &Path, event: &SseEvent) -> Option<&'static str> {
    let filename = "upstream_terminal_event.sse";
    let mut frame = String::new();
    if !event.is_default_event_type() {
        frame.push_str("event: ");
        frame.push_str(&event.event_type);
        frame.push('\n');
    }
    frame.push_str("data: ");
    frame.push_str(&event.data);
    frame.push_str("\n\n");
    fs::write(bundle_dir.join(filename), frame)
        .ok()
        .map(|()| filename)
}

#[derive(Serialize)]
struct DiagnosticUpstream {
    status: u16,
    content_type: String,
    ttfb_ms: u64,
    duration_ms: u128,
    chunks: u64,
    bytes: u64,
}

impl DiagnosticUpstream {
    fn from_openai_responses(snapshot: &ResponsesUpstreamStreamSnapshot) -> Self {
        Self {
            status: snapshot.head.status.as_u16(),
            content_type: snapshot.head.content_type_text(),
            ttfb_ms: snapshot.head.ttfb.as_millis() as u64,
            duration_ms: snapshot.metrics.duration_ms(),
            chunks: snapshot.metrics.chunks,
            bytes: snapshot.metrics.bytes,
        }
    }
}

#[derive(Serialize)]
struct OpenaiResponsesDiagnosticResponse {
    id: Option<String>,
    model: Option<String>,
    status: Option<String>,
    sequence_number: Option<u64>,
    snapshot_kind: Option<String>,
    error: ErrorObject,
    summary: ResponseSummary,
}

#[derive(Serialize)]
struct AnthropicDiagnosticResponse {
    model: Option<String>,
    stop_reason: Option<String>,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    output_items: std::collections::BTreeMap<String, u64>,
}

#[derive(Serialize)]
struct DiagnosticArtifacts {
    terminal_event: Option<&'static str>,
}

#[cfg(test)]
#[path = "provider_semantic_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "provider_semantic_regression_tests.rs"]
mod regression_tests;
