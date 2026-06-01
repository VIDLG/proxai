use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use axum::http::{HeaderMap, Method, StatusCode, Uri};

use crate::config::CaptureConfig;
use crate::error::Result;
use crate::upstream::{ContentType, UpstreamResponseHead};

use super::model::{
    CaptureDestination, CaptureQuery, CaptureRecord, CaptureShowTarget, ForwardedRequestArtifacts,
    ForwardedRequestCapture, InboundRequestCapture, OutboundResponseArtifacts,
    UpstreamResponseArtifacts,
};
use super::write::{
    capture_forwarded_request, capture_inbound_request, capture_outbound_response_headers,
    capture_upstream_response_body, capture_upstream_response_headers,
    UpstreamResponseCaptureWriter,
};

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
    pub forwarded_request_enabled: Option<bool>,
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
}

impl CaptureController {
    pub fn new(dir: Option<PathBuf>, defaults: CaptureConfig) -> Self {
        Self {
            dir,
            defaults,
            runtime: Arc::new(RwLock::new(CaptureRuntimeState::default())),
            records: Arc::new(RwLock::new(VecDeque::new())),
        }
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
            forwarded_request_enabled: runtime
                .overrides
                .forwarded_request_enabled
                .unwrap_or(self.defaults.forwarded_request_enabled),
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
    pub fn set_forwarded_request_enabled_override(&self, enabled: Option<bool>) {
        let mut runtime = self.runtime.write().expect("capture runtime lock poisoned");
        runtime.overrides.forwarded_request_enabled = enabled;
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

    pub fn session(&self, request_id: u64) -> CaptureSession {
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

    pub fn apply_directive(&self, directive: CaptureDirective) {
        let mut runtime = self.runtime.write().expect("capture runtime lock poisoned");
        match directive {
            CaptureDirective::Start => {
                runtime.mode = CaptureSessionMode::Active;
                runtime.overrides.inbound_request_enabled = Some(true);
                runtime.overrides.forwarded_request_enabled = Some(true);
                runtime.overrides.upstream_response_enabled = Some(true);
                runtime.overrides.outbound_response_enabled = Some(true);
            }
            CaptureDirective::Stop => {
                runtime.mode = CaptureSessionMode::Inactive;
                runtime.overrides.inbound_request_enabled = Some(false);
                runtime.overrides.forwarded_request_enabled = Some(false);
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
    pub fn record_for_request(&self, request_id: u64) -> Option<CaptureRecord> {
        self.records
            .read()
            .expect("capture records lock poisoned")
            .iter()
            .find(|record| record.request_id == request_id)
            .cloned()
    }

    fn update_record(&self, request_id: u64, prefix: String, f: impl FnOnce(&mut CaptureRecord)) {
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
                CaptureShowTarget::ForwardedRequest
                    | CaptureShowTarget::UpstreamResponse
                    | CaptureShowTarget::OutboundResponse
            )
        ) {
            if let Some(inbound_request) = record.inbound_request.as_ref() {
                lines.push(format!(
                    "inbound_request.metadata: {}",
                    inbound_request.metadata_path.display()
                ));
                lines.push(format!(
                    "inbound_request.body: {}",
                    inbound_request.body_path.display()
                ));
            }
        }

        if !matches!(
            target,
            Some(
                CaptureShowTarget::InboundRequest
                    | CaptureShowTarget::UpstreamResponse
                    | CaptureShowTarget::OutboundResponse
            )
        ) {
            if let Some(forwarded_request) = record.forwarded_request.as_ref() {
                lines.push(format!(
                    "forwarded_request.metadata: {}",
                    forwarded_request.metadata_path.display()
                ));
                lines.push(format!(
                    "forwarded_request.body: {}",
                    forwarded_request.body_path.display()
                ));
            }
        }

        if !matches!(
            target,
            Some(
                CaptureShowTarget::InboundRequest
                    | CaptureShowTarget::ForwardedRequest
                    | CaptureShowTarget::OutboundResponse
            )
        ) {
            if let Some(upstream_response) = record.upstream_response.as_ref() {
                if let Some(path) = upstream_response.headers_path.as_ref() {
                    lines.push(format!("upstream_response.headers: {}", path.display()));
                }
                if let Some(path) = upstream_response.body_path.as_ref() {
                    lines.push(format!("upstream_response.body: {}", path.display()));
                }
            }
        }

        if !matches!(
            target,
            Some(
                CaptureShowTarget::InboundRequest
                    | CaptureShowTarget::ForwardedRequest
                    | CaptureShowTarget::UpstreamResponse
            )
        ) {
            if let Some(outbound_response) = record.outbound_response.as_ref() {
                if let Some(path) = outbound_response.headers_path.as_ref() {
                    lines.push(format!("outbound_response.headers: {}", path.display()));
                }
                if let Some(path) = outbound_response.body_path.as_ref() {
                    lines.push(format!("outbound_response.body: {}", path.display()));
                }
            }
        }

        lines.join("\n")
    }
}

#[derive(Debug, Clone)]
pub struct CaptureSession {
    controller: CaptureController,
    request_id: u64,
    config: CaptureConfig,
    destination: Option<CaptureDestination>,
}

impl CaptureSession {
    pub fn config(&self) -> CaptureConfig {
        self.config
    }

    pub fn forwarded_request_enabled(&self) -> bool {
        self.config.forwarded_request_enabled && self.destination.is_some()
    }

    pub(crate) fn destination_for_upstream_response(&self) -> Option<&CaptureDestination> {
        self.config
            .upstream_response_enabled
            .then_some(self.destination.as_ref())
            .flatten()
    }

    pub(crate) async fn capture_inbound_request(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<()> {
        if !self.config.inbound_request_enabled {
            return Ok(());
        }
        let Some(destination) = self.destination.as_ref() else {
            return Ok(());
        };
        let record = capture_inbound_request(
            destination,
            InboundRequestCapture {
                request_id: self.request_id,
                method,
                uri,
                headers,
                body,
            },
        )
        .await?;
        self.controller
            .update_record(record.request_id, record.prefix, |entry| {
                entry.inbound_request = Some(super::model::InboundRequestArtifacts {
                    metadata_path: record.metadata_path,
                    body_path: record.body_path,
                });
            });
        Ok(())
    }

    pub(crate) async fn capture_forwarded_request(
        &self,
        method: &Method,
        url: &str,
        headers: &HeaderMap,
        body: &[u8],
        normalized_payload: Option<&serde_json::Value>,
    ) -> Result<()> {
        if !self.config.forwarded_request_enabled {
            return Ok(());
        }
        let Some(destination) = self.destination.as_ref() else {
            return Ok(());
        };
        let record = capture_forwarded_request(
            destination,
            ForwardedRequestCapture {
                request_id: self.request_id,
                method,
                url,
                headers,
                body,
                normalized_payload,
            },
        )
        .await?;
        self.controller
            .update_record(record.request_id, record.prefix, |entry| {
                entry.forwarded_request = Some(ForwardedRequestArtifacts {
                    metadata_path: record.metadata_path,
                    body_path: record.body_path,
                });
            });
        Ok(())
    }

    pub(crate) async fn capture_upstream_response_headers(
        &self,
        head: &UpstreamResponseHead,
        headers: &HeaderMap,
    ) -> Result<()> {
        let Some(destination) = self.destination_for_upstream_response() else {
            return Ok(());
        };
        let path =
            capture_upstream_response_headers(destination, self.request_id, head, headers).await?;
        self.controller
            .update_record(self.request_id, destination.prefix_string(), |entry| {
                entry
                    .upstream_response
                    .get_or_insert_with(UpstreamResponseArtifacts::default)
                    .headers_path = Some(path);
            });
        Ok(())
    }

    pub(crate) async fn capture_upstream_response_body(
        &self,
        content_type: Option<&ContentType>,
        body: &[u8],
    ) -> Result<()> {
        let Some(destination) = self.destination_for_upstream_response() else {
            return Ok(());
        };
        let path = capture_upstream_response_body(destination, content_type, body).await?;
        self.controller
            .update_record(self.request_id, destination.prefix_string(), |entry| {
                entry
                    .upstream_response
                    .get_or_insert_with(UpstreamResponseArtifacts::default)
                    .body_path = Some(path);
            });
        Ok(())
    }

    pub(crate) fn create_upstream_response_writer(
        &self,
        content_type: Option<&ContentType>,
    ) -> Option<UpstreamResponseCaptureWriter> {
        let destination = self.destination_for_upstream_response()?;
        UpstreamResponseCaptureWriter::create(destination, content_type)
            .ok()
            .inspect(|writer| {
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
            })
    }

    pub(crate) async fn capture_outbound_response_headers(
        &self,
        status: StatusCode,
        content_type: Option<&str>,
        headers: &HeaderMap,
    ) -> Result<()> {
        if !self.config.outbound_response_enabled {
            return Ok(());
        }
        let Some(destination) = self.destination.as_ref() else {
            return Ok(());
        };
        let path = capture_outbound_response_headers(
            destination,
            self.request_id,
            status,
            content_type,
            headers,
        )
        .await?;
        self.controller
            .update_record(self.request_id, destination.prefix_string(), |entry| {
                entry
                    .outbound_response
                    .get_or_insert_with(OutboundResponseArtifacts::default)
                    .headers_path = Some(path);
            });
        Ok(())
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
                let forwarded_request = record.forwarded_request.is_some();
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
                    "request_id={} prefix={} inbound_request={} forwarded_request={} upstream_response={} outbound_response={}",
                    record.request_id,
                    record.prefix,
                    inbound_request,
                    forwarded_request,
                    upstream_response,
                    outbound_response
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
