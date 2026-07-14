use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::observe::point::RequestTranslationFailure;
use crate::paths;
use crate::request::RequestId;
use crate::translation::TranslationError;

const KIND: &str = "request_translation_failure";
const PHASE: &str = "provider_request";

pub(super) fn write_request_translation_failure(
    request_id: RequestId,
    point: &RequestTranslationFailure<'_>,
) -> Option<PathBuf> {
    let diagnostics_dir = paths::ensure_app_paths().ok()?.diagnostics_dir;
    write_request_translation_failure_to_dir(request_id, point, &diagnostics_dir)
}

fn write_request_translation_failure_to_dir(
    request_id: RequestId,
    point: &RequestTranslationFailure<'_>,
    diagnostics_dir: &Path,
) -> Option<PathBuf> {
    fs::create_dir_all(diagnostics_dir).ok()?;

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let id = format!("{timestamp}-{:06}-{KIND}", request_id.as_u64());
    let bundle_dir = diagnostics_dir.join(&id);
    fs::create_dir_all(&bundle_dir).ok()?;

    let normalized_payload_file = "normalized_payload.json";
    fs::write(
        bundle_dir.join(normalized_payload_file),
        serde_json::to_vec_pretty(point.normalized_payload).ok()?,
    )
    .ok()?;

    let (json_path, line, column) = match point.error {
        TranslationError::JsonPayload {
            path, line, column, ..
        } => (Some(path.as_str()), Some(*line), Some(*column)),
        _ => (None, None, None),
    };
    let error_message = point.error.to_string();
    let record = DiagnosticRecord {
        id,
        created_at: Utc::now().to_rfc3339(),
        request_id: request_id.as_u64(),
        kind: KIND,
        phase: PHASE,
        summary: "Failed to translate the normalized inbound request to the selected provider protocol",
        route: DiagnosticRoute {
            request_protocol: point.request_protocol.to_string(),
            provider: point.provider,
            route_name: point.route_name,
            provider_protocol: point.provider_protocol.to_string(),
            model: point.model,
        },
        error: DiagnosticError {
            message: &error_message,
            json_path,
            line,
            column,
        },
        artifacts: DiagnosticArtifacts {
            normalized_payload: normalized_payload_file,
        },
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
    route: DiagnosticRoute<'a>,
    error: DiagnosticError<'a>,
    artifacts: DiagnosticArtifacts<'a>,
}

#[derive(Serialize)]
struct DiagnosticRoute<'a> {
    request_protocol: String,
    provider: &'a str,
    route_name: Option<&'a str>,
    provider_protocol: String,
    model: &'a str,
}

#[derive(Serialize)]
struct DiagnosticError<'a> {
    message: &'a str,
    json_path: Option<&'a str>,
    line: Option<usize>,
    column: Option<usize>,
}

#[derive(Serialize)]
struct DiagnosticArtifacts<'a> {
    normalized_payload: &'a str,
}

#[cfg(test)]
#[path = "translation_tests.rs"]
mod tests;
