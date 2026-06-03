mod openai_responses;

use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths;
use crate::provider::openai::responses::ResponsesUpstreamStreamSnapshot;
use crate::request::RequestId;

pub(crate) use openai_responses::OpenaiResponsesStreamDiagnostics;

#[derive(Clone)]
pub(crate) struct DiagnosticsSink {
    request_id: RequestId,
    openai_responses_stream: Arc<Mutex<OpenaiResponsesStreamDiagnostics>>,
}

impl DiagnosticsSink {
    pub(crate) fn new(request_id: RequestId) -> Self {
        Self {
            request_id,
            openai_responses_stream: Arc::new(Mutex::new(OpenaiResponsesStreamDiagnostics::new(
                request_id,
            ))),
        }
    }

    pub(crate) fn record_request_info_parse_failure(
        &self,
        normalized_payload: &Value,
        request_info_parse_payload: &Value,
        error: &serde_json::Error,
    ) -> Option<PathBuf> {
        write_request_info_parse_failure(
            self.request_id.as_u64(),
            normalized_payload,
            request_info_parse_payload,
            error,
        )
    }

    pub(crate) fn observe_openai_responses_stream_chunk(&self, chunk: &[u8]) {
        self.openai_responses_stream
            .lock()
            .expect("openai responses stream diagnostics lock poisoned")
            .observe_chunk(chunk);
    }

    pub(crate) fn record_openai_responses_unfinished_tool_stream(
        &self,
        snapshot: &ResponsesUpstreamStreamSnapshot,
    ) -> Option<String> {
        self.openai_responses_stream
            .lock()
            .expect("openai responses stream diagnostics lock poisoned")
            .write_unfinished_tool_diagnostic(snapshot)
    }
}

const MAX_RECORDS: usize = 50;
const KIND_REQUEST_INFO_PARSE_FAILURE: &str = "request_info_parse_failure";
const PHASE_FORWARDED_REQUEST: &str = "provider_request";

#[derive(Debug, Serialize)]
struct DiagnosticArtifacts<'a> {
    normalized_payload: &'a str,
    request_info_parse_payload: &'a str,
}

#[derive(Debug, Serialize)]
struct DiagnosticError<'a> {
    message: &'a str,
}

#[derive(Debug, Serialize)]
struct DiagnosticRecord<'a> {
    id: String,
    created_at: String,
    request_id: u64,
    kind: &'a str,
    phase: &'a str,
    summary: &'a str,
    error: DiagnosticError<'a>,
    artifacts: DiagnosticArtifacts<'a>,
}

pub(crate) fn write_request_info_parse_failure(
    request_id: u64,
    normalized_payload: &Value,
    request_info_parse_payload: &Value,
    error: &serde_json::Error,
) -> Option<PathBuf> {
    let app_paths = paths::ensure_app_paths().ok()?;
    let diagnostics_dir = app_paths.diagnostics_dir;
    fs::create_dir_all(&diagnostics_dir).ok()?;

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let id = format!("{timestamp}-{request_id:06}-{KIND_REQUEST_INFO_PARSE_FAILURE}");
    let bundle_dir = diagnostics_dir.join(&id);
    fs::create_dir_all(&bundle_dir).ok()?;

    let normalized_payload_file = "normalized_payload.json";
    let request_info_parse_payload_file = "request_info_parse_payload.json";
    let record_file = "record.json";

    fs::write(
        bundle_dir.join(normalized_payload_file),
        serde_json::to_vec_pretty(normalized_payload).ok()?,
    )
    .ok()?;
    fs::write(
        bundle_dir.join(request_info_parse_payload_file),
        serde_json::to_vec_pretty(request_info_parse_payload).ok()?,
    )
    .ok()?;

    let error_message = error.to_string();
    let record = DiagnosticRecord {
        id,
        created_at: Utc::now().to_rfc3339(),
        request_id,
        kind: KIND_REQUEST_INFO_PARSE_FAILURE,
        phase: PHASE_FORWARDED_REQUEST,
        summary: "Failed to extract RequestInfo from forwarded OpenAI Responses payload",
        error: DiagnosticError {
            message: &error_message,
        },
        artifacts: DiagnosticArtifacts {
            normalized_payload: normalized_payload_file,
            request_info_parse_payload: request_info_parse_payload_file,
        },
    };
    fs::write(
        bundle_dir.join(record_file),
        serde_json::to_vec_pretty(&record).ok()?,
    )
    .ok()?;

    trim_old_records(&diagnostics_dir);
    Some(bundle_dir)
}

fn trim_old_records(diagnostics_dir: &Path) {
    let Ok(entries) = fs::read_dir(diagnostics_dir) else {
        return;
    };

    let mut dirs = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            path.is_dir().then_some(path)
        })
        .collect::<Vec<_>>();
    if dirs.len() <= MAX_RECORDS {
        return;
    }

    dirs.sort();
    let remove_count = dirs.len().saturating_sub(MAX_RECORDS);
    for path in dirs.into_iter().take(remove_count) {
        let _ = fs::remove_dir_all(path);
    }
}
