use std::time::Instant;

use axum::http::{HeaderMap, Method, Uri};

use crate::capture::CaptureSession;
use crate::config::ErrorResponseFormat;
use crate::request::RequestId;

mod inbound;
mod provider_request;
mod provider_response;
mod run;
mod upstream_response;

pub(crate) use inbound::InboundHttpFlow;
pub(crate) use run::run_provider_flow;

pub(crate) struct ProxyFlow<S> {
    pub(super) method: Method,
    pub(super) uri: Uri,
    pub(super) headers: HeaderMap,
    pub(super) request_id: RequestId,
    pub(super) started: Instant,
    pub(super) span: tracing::Span,
    pub(super) capture: CaptureSession,
    pub(super) error_response_format: ErrorResponseFormat,
    pub(super) stage: S,
}
