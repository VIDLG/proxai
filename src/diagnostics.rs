use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tracing::debug;

use crate::paths;

/// Write a diagnostic file when a `/v1/responses` request payload fails to
/// parse into a `RequestProjection`. Returns the path on success.
pub(crate) fn write_request_info_parse_failure(
    request_id: u64,
    original: &Value,
    adapted: &Value,
    error: &serde_json::Error,
) -> Option<PathBuf> {
    let diagnostics_dir = paths::ensure_app_paths().ok()?.diagnostics_dir;
    fs::create_dir_all(&diagnostics_dir).ok()?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    let path = diagnostics_dir.join(format!(
        "request-parse-failure-{timestamp}-{request_id:06}.json"
    ));

    let diagnostic = serde_json::json!({
        "request_id": request_id,
        "error": error.to_string(),
        "original_payload": original,
        "adapted_payload": adapted,
    });

    match fs::write(&path, serde_json::to_vec_pretty(&diagnostic).ok()?) {
        Ok(()) => {
            debug!(
                request_id = request_id,
                path = %path.display(),
                "wrote request parse failure diagnostic"
            );
            Some(path)
        }
        Err(write_err) => {
            debug!(
                request_id = request_id,
                error = %write_err,
                "failed to write request parse failure diagnostic"
            );
            None
        }
    }
}
