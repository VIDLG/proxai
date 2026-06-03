use std::fs::{self as stdfs, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use axum::http::HeaderMap;
use serde_json::Value;
use tokio::fs;
use tracing::info;

use crate::error::Result;
use crate::http_model::UpstreamResponseHead;
use crate::http_utils::ContentType;
use crate::request::RequestId;

use super::model::{CaptureDestination, InboundRequestCapture, ProviderRequestCapture};

pub(crate) struct InboundRequestCaptureArtifacts {
    pub(crate) request_id: RequestId,
    pub(crate) prefix: String,
    pub(crate) metadata_path: PathBuf,
    pub(crate) body_path: PathBuf,
}

pub(crate) struct ProviderRequestCaptureArtifacts {
    pub(crate) request_id: RequestId,
    pub(crate) prefix: String,
    pub(crate) metadata_path: PathBuf,
    pub(crate) body_path: PathBuf,
}

pub(crate) async fn capture_inbound_request(
    destination: &CaptureDestination,
    request: InboundRequestCapture<'_>,
) -> Result<InboundRequestCaptureArtifacts> {
    let metadata_path = destination.inbound_request_metadata_path();
    let body_path = destination.inbound_request_body_path();
    ensure_parent_dir(&metadata_path).await?;

    let metadata = serde_json::json!({
        "request_id": request.request_id,
        "method": request.method.as_str(),
        "path": request.uri.to_string(),
        "headers": sanitized_headers(request.headers),
        "inbound_request_body_bytes": request.body.len(),
    });
    fs::write(&metadata_path, serde_json::to_vec_pretty(&metadata)?).await?;
    fs::write(&body_path, pretty_json_or_raw(request.body)?).await?;
    info!(
        request_id = %request.request_id,
        event = "capture",
        kind = "inbound_request",
        metadata_path = %metadata_path.display(),
        body_path = %body_path.display(),
        "capture saved"
    );
    Ok(InboundRequestCaptureArtifacts {
        request_id: request.request_id,
        prefix: destination.prefix_string(),
        metadata_path,
        body_path,
    })
}

pub(crate) async fn capture_provider_request(
    destination: &CaptureDestination,
    request: ProviderRequestCapture<'_>,
) -> Result<ProviderRequestCaptureArtifacts> {
    let metadata_path = destination.provider_request_metadata_path();
    let body_path = destination.provider_request_body_path();
    ensure_parent_dir(&metadata_path).await?;

    let metadata = serde_json::json!({
        "request_id": request.request_id,
        "method": request.method.as_str(),
        "url": request.url,
        "headers": sanitized_headers(request.headers),
        "provider_request_body_bytes": request.body.len(),
        "normalized": request.normalized_payload.is_some(),
    });
    fs::write(&metadata_path, serde_json::to_vec_pretty(&metadata)?).await?;
    fs::write(&body_path, pretty_json_or_raw(request.body)?).await?;
    info!(
        request_id = %request.request_id,
        event = "capture",
        kind = "provider_request",
        metadata_path = %metadata_path.display(),
        body_path = %body_path.display(),
        "capture saved"
    );
    Ok(ProviderRequestCaptureArtifacts {
        request_id: request.request_id,
        prefix: destination.prefix_string(),
        metadata_path,
        body_path,
    })
}

pub(crate) async fn capture_upstream_response_headers(
    destination: &CaptureDestination,
    request_id: RequestId,
    head: &UpstreamResponseHead,
) -> Result<PathBuf> {
    let headers_path = destination.upstream_response_headers_path();
    ensure_parent_dir(&headers_path).await?;
    let metadata = serde_json::json!({
        "request_id": request_id,
        "status": head.status.as_u16(),
        "content_type": head.content_type().map(|value| value.to_string()).unwrap_or_default(),
        "content_length": head.content_length(),
        "transfer_encoding": head.transfer_encoding().unwrap_or_default(),
        "ttfb_ms": head.ttfb.as_millis() as u64,
        "headers": sanitized_headers(&head.headers),
    });
    fs::write(&headers_path, serde_json::to_vec_pretty(&metadata)?).await?;
    info!(
        request_id = %request_id,
        event = "capture",
        kind = "upstream_response_headers",
        status = head.status.as_u16(),
        path = %headers_path.display(),
        "capture saved"
    );
    Ok(headers_path)
}

pub(crate) async fn capture_upstream_response_body(
    destination: &CaptureDestination,
    content_type: Option<&ContentType>,
    body: &[u8],
) -> Result<PathBuf> {
    let body_path = destination.upstream_response_body_path(content_type);
    ensure_parent_dir(&body_path).await?;
    fs::write(&body_path, body).await?;
    info!(
        event = "capture",
        kind = "upstream_response_body",
        content_type = content_type.map(ToString::to_string).unwrap_or_default(),
        bytes = body.len(),
        path = %body_path.display(),
        "capture saved"
    );
    Ok(body_path)
}

pub(crate) async fn capture_outbound_response_headers(
    destination: &CaptureDestination,
    request_id: RequestId,
    status: axum::http::StatusCode,
    content_type: Option<&str>,
    headers: &HeaderMap,
) -> Result<PathBuf> {
    let headers_path = destination.outbound_response_headers_path();
    ensure_parent_dir(&headers_path).await?;
    let metadata = serde_json::json!({
        "request_id": request_id,
        "status": status.as_u16(),
        "content_type": content_type,
        "headers": sanitized_headers(headers),
    });
    fs::write(&headers_path, serde_json::to_vec_pretty(&metadata)?).await?;
    info!(
        request_id = %request_id,
        event = "capture",
        kind = "outbound_response_headers",
        status = status.as_u16(),
        path = %headers_path.display(),
        "capture saved"
    );
    Ok(headers_path)
}

pub struct UpstreamResponseCaptureWriter {
    file: File,
    path: PathBuf,
}

impl UpstreamResponseCaptureWriter {
    pub(crate) fn create(
        destination: &CaptureDestination,
        content_type: Option<&ContentType>,
    ) -> std::io::Result<Self> {
        let body_path = destination.upstream_response_body_path(content_type);
        if let Some(parent) = body_path.parent() {
            stdfs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&body_path)?;
        info!(
            event = "capture",
            kind = "upstream_response_stream",
            content_type = content_type.map(ToString::to_string).unwrap_or_default(),
            path = %body_path.display(),
            "capture started"
        );
        Ok(Self {
            file,
            path: body_path,
        })
    }

    pub fn write_chunk(&mut self, chunk: &[u8]) {
        let _ = self.file.write_all(chunk);
        let _ = self.file.flush();
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

async fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    Ok(())
}

fn pretty_json_or_raw(bytes: &[u8]) -> Result<Vec<u8>> {
    if let Ok(value) = serde_json::from_slice::<Value>(bytes) {
        return Ok(serde_json::to_vec_pretty(&value)?);
    }
    Ok(bytes.to_vec())
}

fn sanitized_headers(headers: &HeaderMap) -> Value {
    let entries = headers
        .iter()
        .map(|(key, value)| {
            let name = key.as_str();
            let value = if is_sensitive_header(name) {
                "<redacted>".to_string()
            } else {
                value.to_str().unwrap_or("<non-utf8>").to_string()
            };
            (name.to_string(), Value::String(value))
        })
        .collect();
    Value::Object(entries)
}

fn is_sensitive_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization" | "cookie" | "set-cookie" | "proxy-authorization" | "x-api-key"
    )
}
