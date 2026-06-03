use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use axum::body::Bytes;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use tokio::sync::{mpsc, oneshot};
use tracing::warn;

use crate::config::CaptureConfig;

use crate::http_support::ContentType;
use crate::http_support::UpstreamResponseHead;
use crate::request::RequestId;

use super::model::{
    CaptureDestination, CaptureQuery, CaptureRecord, CaptureShowTarget, InboundRequestCapture,
    OutboundResponseArtifacts, ProviderRequestArtifacts, ProviderRequestCapture,
    UpstreamResponseArtifacts,
};
use super::write::{
    UpstreamResponseCaptureWriter, capture_inbound_request, capture_outbound_response_headers,
    capture_provider_request, capture_upstream_response_body, capture_upstream_response_headers,
};

const CAPTURE_QUEUE_CAPACITY: usize = 1024;

#[derive(Debug)]
enum CaptureJob {
    InboundRequest {
        request_id: RequestId,
        destination: CaptureDestination,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Bytes,
    },
    ProviderRequest {
        request_id: RequestId,
        destination: CaptureDestination,
        method: Method,
        url: String,
        headers: HeaderMap,
        body: Bytes,
        normalized_payload: Option<serde_json::Value>,
    },
    UpstreamResponseHeaders {
        request_id: RequestId,
        destination: CaptureDestination,
        head: UpstreamResponseHead,
    },
    UpstreamResponseBody {
        request_id: RequestId,
        destination: CaptureDestination,
        content_type: Option<ContentType>,
        body: Bytes,
    },
    OutboundResponseHeaders {
        request_id: RequestId,
        destination: CaptureDestination,
        status: StatusCode,
        content_type: Option<String>,
        headers: HeaderMap,
    },
    Flush {
        done: oneshot::Sender<()>,
    },
}

#[derive(Debug, Clone)]
pub struct CaptureStatus {
    pub defaults: CaptureConfig,
    pub overrides: CaptureOverrides,
    pub effective: CaptureConfig,
    pub captures_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CaptureOverrides {
    pub inbound_request_enabled: Option<bool>,
    pub provider_request_enabled: Option<bool>,
    pub upstream_response_enabled: Option<bool>,
    pub outbound_response_enabled: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureDirective {
    Start,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureSessionMode {
    Inactive,
    Active,
}

#[derive(Debug, Clone, Copy)]
struct CaptureRuntimeState {
    overrides: CaptureOverrides,
    mode: CaptureSessionMode,
}

impl Default for CaptureRuntimeState {
    fn default() -> Self {
        Self {
            overrides: CaptureOverrides::default(),
            mode: CaptureSessionMode::Inactive,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CaptureController {
    dir: Option<PathBuf>,
    defaults: CaptureConfig,
    runtime: Arc<RwLock<CaptureRuntimeState>>,
    records: Arc<RwLock<VecDeque<CaptureRecord>>>,
    jobs: mpsc::Sender<CaptureJob>,
}

impl CaptureController {
    pub fn new(dir: Option<PathBuf>, defaults: CaptureConfig) -> Self {
        let (jobs, rx) = mpsc::channel(CAPTURE_QUEUE_CAPACITY);
        let controller = Self {
            dir,
            defaults,
            runtime: Arc::new(RwLock::new(CaptureRuntimeState::default())),
            records: Arc::new(RwLock::new(VecDeque::new())),
            jobs,
        };
        controller.spawn_worker(rx);
        controller
    }

    #[allow(dead_code)]
    pub fn set_overrides(&self, overrides: CaptureOverrides) {
        self.runtime
            .write()
            .expect("capture runtime lock poisoned")
            .overrides = overrides;
    }

    #[allow(dead_code)]
    pub fn clear_overrides(&self) {
        self.runtime
            .write()
            .expect("capture runtime lock poisoned")
            .overrides = CaptureOverrides::default();
    }

    pub fn effective_config(&self) -> CaptureConfig {
        let runtime = *self.runtime.read().expect("capture runtime lock poisoned");
        CaptureConfig {
            inbound_request_enabled: runtime
                .overrides
                .inbound_request_enabled
                .unwrap_or(self.defaults.inbound_request_enabled),
            provider_request_enabled: runtime
                .overrides
                .provider_request_enabled
                .unwrap_or(self.defaults.provider_request_enabled),
            upstream_response_enabled: runtime
                .overrides
                .upstream_response_enabled
                .unwrap_or(self.defaults.upstream_response_enabled),
            outbound_response_enabled: runtime
                .overrides
                .outbound_response_enabled
                .unwrap_or(self.defaults.outbound_response_enabled),
        }
    }

    #[allow(dead_code)]
    pub fn overrides(&self) -> CaptureOverrides {
        self.runtime
            .read()
            .expect("capture runtime lock poisoned")
            .overrides
    }

    #[allow(dead_code)]
    pub fn status(&self) -> CaptureStatus {
        let runtime = *self.runtime.read().expect("capture runtime lock poisoned");
        CaptureStatus {
            defaults: self.defaults,
            overrides: runtime.overrides,
            effective: self.effective_config(),
            captures_dir: self.dir.clone(),
        }
    }

    pub fn default_config(&self) -> CaptureConfig {
        self.defaults
    }

    #[allow(dead_code)]
    pub fn set_inbound_request_enabled_override(&self, enabled: Option<bool>) {
        let mut runtime = self.runtime.write().expect("capture runtime lock poisoned");
        runtime.overrides.inbound_request_enabled = enabled;
    }

    #[allow(dead_code)]
    pub fn set_provider_request_enabled_override(&self, enabled: Option<bool>) {
        let mut runtime = self.runtime.write().expect("capture runtime lock poisoned");
        runtime.overrides.provider_request_enabled = enabled;
    }

    #[allow(dead_code)]
    pub fn set_upstream_response_enabled_override(&self, enabled: Option<bool>) {
        let mut runtime = self.runtime.write().expect("capture runtime lock poisoned");
        runtime.overrides.upstream_response_enabled = enabled;
    }

    #[allow(dead_code)]
    pub fn set_outbound_response_enabled_override(&self, enabled: Option<bool>) {
        let mut runtime = self.runtime.write().expect("capture runtime lock poisoned");
        runtime.overrides.outbound_response_enabled = enabled;
    }

    pub fn set_default_config(&mut self, defaults: CaptureConfig) {
        self.defaults = defaults;
    }

    pub fn set_dir(&mut self, dir: Option<PathBuf>) {
        self.dir = dir;
    }

    pub fn session(&self, request_id: RequestId) -> CaptureSession {
        let config = self.effective_config();
        let destination = self
            .dir
            .as_ref()
            .filter(|_| config.any_enabled())
            .map(|dir| {
                CaptureDestination::new(dir.clone(), super::model::CapturePrefix::new(request_id))
            });
        CaptureSession {
            controller: self.clone(),
            request_id,
            config,
            destination,
        }
    }

    pub fn render_query(&self, query: &CaptureQuery) -> String {
        match query {
            CaptureQuery::Show(target) => self.render_latest(target.as_ref()),
            CaptureQuery::List(limit) => self.render_list(limit.unwrap_or(10)),
        }
    }

    pub async fn flush(&self) {
        let (done, wait) = oneshot::channel();
        if self.jobs.send(CaptureJob::Flush { done }).await.is_ok() {
            let _ = wait.await;
        }
    }

    fn enqueue(&self, job: CaptureJob) {
        if let Err(error) = self.jobs.try_send(job) {
            warn!(
                event = "capture_drop",
                reason = %error,
                "capture job dropped"
            );
        }
    }

    fn spawn_worker(&self, mut rx: mpsc::Receiver<CaptureJob>) {
        let controller = self.clone();
        let worker = async move {
            while let Some(job) = rx.recv().await {
                controller.handle_job(job).await;
            }
        };
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(worker);
        } else {
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("capture worker runtime should build");
                runtime.block_on(worker);
            });
        }
    }

    async fn handle_job(&self, job: CaptureJob) {
        match job {
            CaptureJob::InboundRequest {
                request_id,
                destination,
                method,
                uri,
                headers,
                body,
            } => {
                let record = match capture_inbound_request(
                    &destination,
                    InboundRequestCapture {
                        request_id,
                        method: &method,
                        uri: &uri,
                        headers: &headers,
                        body: &body,
                    },
                )
                .await
                {
                    Ok(record) => record,
                    Err(error) => {
                        log_capture_failure(request_id, "inbound_request", &error);
                        return;
                    }
                };
                self.update_record(record.request_id, record.prefix, |entry| {
                    entry.inbound_request = Some(super::model::InboundRequestArtifacts {
                        metadata_path: record.metadata_path,
                        body_path: record.body_path,
                    });
                });
            }
            CaptureJob::ProviderRequest {
                request_id,
                destination,
                method,
                url,
                headers,
                body,
                normalized_payload,
            } => {
                let record = match capture_provider_request(
                    &destination,
                    ProviderRequestCapture {
                        request_id,
                        method: &method,
                        url: &url,
                        headers: &headers,
                        body: &body,
                        normalized_payload: normalized_payload.as_ref(),
                    },
                )
                .await
                {
                    Ok(record) => record,
                    Err(error) => {
                        log_capture_failure(request_id, "provider_request", &error);
                        return;
                    }
                };
                self.update_record(record.request_id, record.prefix, |entry| {
                    entry.provider_request = Some(ProviderRequestArtifacts {
                        metadata_path: record.metadata_path,
                        body_path: record.body_path,
                    });
                });
            }
            CaptureJob::UpstreamResponseHeaders {
                request_id,
                destination,
                head,
            } => {
                let path = match capture_upstream_response_headers(&destination, request_id, &head)
                    .await
                {
                    Ok(path) => path,
                    Err(error) => {
                        log_capture_failure(request_id, "upstream_response_headers", &error);
                        return;
                    }
                };
                self.update_record(request_id, destination.prefix_string(), |entry| {
                    entry
                        .upstream_response
                        .get_or_insert_with(UpstreamResponseArtifacts::default)
                        .headers_path = Some(path);
                });
            }
            CaptureJob::UpstreamResponseBody {
                request_id,
                destination,
                content_type,
                body,
            } => {
                let path = match capture_upstream_response_body(
                    &destination,
                    content_type.as_ref(),
                    &body,
                )
                .await
                {
                    Ok(path) => path,
                    Err(error) => {
                        log_capture_failure(request_id, "upstream_response_body", &error);
                        return;
                    }
                };
                self.update_record(request_id, destination.prefix_string(), |entry| {
                    entry
                        .upstream_response
                        .get_or_insert_with(UpstreamResponseArtifacts::default)
                        .body_path = Some(path);
                });
            }
            CaptureJob::OutboundResponseHeaders {
                request_id,
                destination,
                status,
                content_type,
                headers,
            } => {
                let path = match capture_outbound_response_headers(
                    &destination,
                    request_id,
                    status,
                    content_type.as_deref(),
                    &headers,
                )
                .await
                {
                    Ok(path) => path,
                    Err(error) => {
                        log_capture_failure(request_id, "outbound_response_headers", &error);
                        return;
                    }
                };
                self.update_record(request_id, destination.prefix_string(), |entry| {
                    entry
                        .outbound_response
                        .get_or_insert_with(OutboundResponseArtifacts::default)
                        .headers_path = Some(path);
                });
            }
            CaptureJob::Flush { done } => {
                let _ = done.send(());
            }
        }
    }

    pub fn apply_directive(&self, directive: CaptureDirective) {
        let mut runtime = self.runtime.write().expect("capture runtime lock poisoned");
        match directive {
            CaptureDirective::Start => {
                runtime.mode = CaptureSessionMode::Active;
                runtime.overrides.inbound_request_enabled = Some(true);
                runtime.overrides.provider_request_enabled = Some(true);
                runtime.overrides.upstream_response_enabled = Some(true);
                runtime.overrides.outbound_response_enabled = Some(true);
            }
            CaptureDirective::Stop => {
                runtime.mode = CaptureSessionMode::Inactive;
                runtime.overrides.inbound_request_enabled = Some(false);
                runtime.overrides.provider_request_enabled = Some(false);
                runtime.overrides.upstream_response_enabled = Some(false);
                runtime.overrides.outbound_response_enabled = Some(false);
            }
        }
    }

    #[allow(dead_code)]
    pub fn latest_record(&self) -> Option<CaptureRecord> {
        self.records
            .read()
            .expect("capture records lock poisoned")
            .back()
            .cloned()
    }

    #[allow(dead_code)]
    pub fn records(&self) -> Vec<CaptureRecord> {
        self.records
            .read()
            .expect("capture records lock poisoned")
            .iter()
            .cloned()
            .collect()
    }

    #[allow(dead_code)]
    pub fn record_for_request(&self, request_id: RequestId) -> Option<CaptureRecord> {
        self.records
            .read()
            .expect("capture records lock poisoned")
            .iter()
            .find(|record| record.request_id == request_id)
            .cloned()
    }

    fn update_record(
        &self,
        request_id: RequestId,
        prefix: String,
        f: impl FnOnce(&mut CaptureRecord),
    ) {
        const MAX_RECORDS: usize = 128;

        let mut records = self.records.write().expect("capture records lock poisoned");
        if let Some(existing) = records
            .iter_mut()
            .find(|record| record.request_id == request_id)
        {
            f(existing);
            return;
        }

        let mut record = CaptureRecord {
            request_id,
            prefix,
            ..CaptureRecord::default()
        };
        f(&mut record);
        records.push_back(record);
        while records.len() > MAX_RECORDS {
            records.pop_front();
        }
    }

    fn render_latest(&self, target: Option<&CaptureShowTarget>) -> String {
        let Some(record) = self.latest_record() else {
            return "No capture records available.".to_string();
        };

        let mut lines = vec![
            format!("request_id: {}", record.request_id),
            format!("prefix: {}", record.prefix),
        ];

        if !matches!(
            target,
            Some(
                CaptureShowTarget::ProviderRequest
                    | CaptureShowTarget::UpstreamResponse
                    | CaptureShowTarget::OutboundResponse
            )
        ) && let Some(inbound_request) = record.inbound_request.as_ref()
        {
            lines.push(format!(
                "inbound_request.metadata: {}",
                inbound_request.metadata_path.display()
            ));
            lines.push(format!(
                "inbound_request.body: {}",
                inbound_request.body_path.display()
            ));
        }

        if !matches!(
            target,
            Some(
                CaptureShowTarget::InboundRequest
                    | CaptureShowTarget::UpstreamResponse
                    | CaptureShowTarget::OutboundResponse
            )
        ) && let Some(provider_request) = record.provider_request.as_ref()
        {
            lines.push(format!(
                "provider_request.metadata: {}",
                provider_request.metadata_path.display()
            ));
            lines.push(format!(
                "provider_request.body: {}",
                provider_request.body_path.display()
            ));
        }

        if !matches!(
            target,
            Some(
                CaptureShowTarget::InboundRequest
                    | CaptureShowTarget::ProviderRequest
                    | CaptureShowTarget::OutboundResponse
            )
        ) && let Some(upstream_response) = record.upstream_response.as_ref()
        {
            if let Some(path) = upstream_response.headers_path.as_ref() {
                lines.push(format!("upstream_response.headers: {}", path.display()));
            }
            if let Some(path) = upstream_response.body_path.as_ref() {
                lines.push(format!("upstream_response.body: {}", path.display()));
            }
        }

        if !matches!(
            target,
            Some(
                CaptureShowTarget::InboundRequest
                    | CaptureShowTarget::ProviderRequest
                    | CaptureShowTarget::UpstreamResponse
            )
        ) && let Some(outbound_response) = record.outbound_response.as_ref()
        {
            if let Some(path) = outbound_response.headers_path.as_ref() {
                lines.push(format!("outbound_response.headers: {}", path.display()));
            }
            if let Some(path) = outbound_response.body_path.as_ref() {
                lines.push(format!("outbound_response.body: {}", path.display()));
            }
        }

        lines.join("\n")
    }
}

fn log_capture_failure(request_id: RequestId, kind: &'static str, error: &dyn std::fmt::Display) {
    warn!(
        request_id = %request_id,
        event = "capture_failed",
        kind,
        error = %error,
        "capture failed"
    );
}

#[derive(Debug, Clone)]
pub struct CaptureSession {
    controller: CaptureController,
    request_id: RequestId,
    config: CaptureConfig,
    destination: Option<CaptureDestination>,
}

impl CaptureSession {
    pub fn config(&self) -> CaptureConfig {
        self.config
    }

    pub fn provider_request_enabled(&self) -> bool {
        self.config.provider_request_enabled && self.destination.is_some()
    }

    pub(crate) fn destination_for_upstream_response(&self) -> Option<&CaptureDestination> {
        self.config
            .upstream_response_enabled
            .then_some(self.destination.as_ref())
            .flatten()
    }

    pub(crate) fn capture_inbound_request(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: &[u8],
    ) {
        if !self.config.inbound_request_enabled {
            return;
        }
        let Some(destination) = self.destination.clone() else {
            return;
        };
        self.controller.enqueue(CaptureJob::InboundRequest {
            request_id: self.request_id,
            destination,
            method: method.clone(),
            uri: uri.clone(),
            headers: headers.clone(),
            body: Bytes::copy_from_slice(body),
        });
    }

    pub(crate) fn capture_provider_request(
        &self,
        method: &Method,
        url: &str,
        headers: &HeaderMap,
        body: &[u8],
        normalized_payload: Option<&serde_json::Value>,
    ) {
        if !self.config.provider_request_enabled {
            return;
        }
        let Some(destination) = self.destination.clone() else {
            return;
        };
        self.controller.enqueue(CaptureJob::ProviderRequest {
            request_id: self.request_id,
            destination,
            method: method.clone(),
            url: url.to_string(),
            headers: headers.clone(),
            body: Bytes::copy_from_slice(body),
            normalized_payload: normalized_payload.cloned(),
        });
    }

    pub(crate) fn capture_upstream_response(&self, head: &UpstreamResponseHead, body: &[u8]) {
        self.capture_upstream_response_headers(head);
        self.capture_upstream_response_body(head.content_type(), body);
    }

    pub(crate) fn capture_upstream_response_headers(&self, head: &UpstreamResponseHead) {
        let Some(destination) = self.destination_for_upstream_response().cloned() else {
            return;
        };
        self.controller
            .enqueue(CaptureJob::UpstreamResponseHeaders {
                request_id: self.request_id,
                destination,
                head: head.clone(),
            });
    }

    pub(crate) fn capture_upstream_response_body(
        &self,
        content_type: Option<ContentType>,
        body: &[u8],
    ) {
        let Some(destination) = self.destination_for_upstream_response().cloned() else {
            return;
        };
        self.controller.enqueue(CaptureJob::UpstreamResponseBody {
            request_id: self.request_id,
            destination,
            content_type,
            body: Bytes::copy_from_slice(body),
        });
    }

    pub(crate) fn create_upstream_response_writer(
        &self,
        content_type: Option<&ContentType>,
    ) -> Option<UpstreamResponseCaptureWriter> {
        let destination = self.destination_for_upstream_response()?;
        match UpstreamResponseCaptureWriter::create(destination, content_type) {
            Ok(writer) => {
                self.controller.update_record(
                    self.request_id,
                    destination.prefix_string(),
                    |entry| {
                        entry
                            .upstream_response
                            .get_or_insert_with(UpstreamResponseArtifacts::default)
                            .body_path = Some(writer.path().to_path_buf());
                    },
                );
                Some(writer)
            }
            Err(error) => {
                log_capture_failure(self.request_id, "upstream_response_stream", &error);
                None
            }
        }
    }

    pub(crate) fn capture_outbound_response_headers(
        &self,
        status: StatusCode,
        content_type: Option<&str>,
        headers: &HeaderMap,
    ) {
        if !self.config.outbound_response_enabled {
            return;
        }
        let Some(destination) = self.destination.clone() else {
            return;
        };
        self.controller
            .enqueue(CaptureJob::OutboundResponseHeaders {
                request_id: self.request_id,
                destination,
                status,
                content_type: content_type.map(ToString::to_string),
                headers: headers.clone(),
            });
    }
}

impl CaptureController {
    fn render_list(&self, limit: usize) -> String {
        let records = self.records();
        if records.is_empty() {
            return "No capture records available.".to_string();
        }

        records
            .iter()
            .rev()
            .take(limit)
            .map(|record| {
                let inbound_request = record.inbound_request.is_some();
                let provider_request = record.provider_request.is_some();
                let upstream_response = record
                    .upstream_response
                    .as_ref()
                    .map(|group| group.headers_path.is_some() || group.body_path.is_some())
                    .unwrap_or(false);
                let outbound_response = record
                    .outbound_response
                    .as_ref()
                    .map(|group| group.headers_path.is_some() || group.body_path.is_some())
                    .unwrap_or(false);
                format!(
                    "request_id={} prefix={} inbound_request={} provider_request={} upstream_response={} outbound_response={}",
                    record.request_id,
                    record.prefix,
                    inbound_request,
                    provider_request,
                    upstream_response,
                    outbound_response
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
