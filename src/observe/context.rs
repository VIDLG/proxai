use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use getset::CopyGetters;
use tracing::{Instrument, info_span};

use super::capture::{CaptureController, CaptureSession, UpstreamResponseCaptureWriter};
use super::diagnostics::DiagnosticsSink;
use crate::request::RequestId;

#[derive(Clone, CopyGetters)]
pub(crate) struct ObserveContext {
    #[getset(get_copy = "pub(crate)")]
    pub(super) request_id: RequestId,
    #[getset(get_copy = "pub(crate)")]
    pub(super) started: Instant,
    pub(super) capture: CaptureSession,
    pub(super) stream_capture_writer: Arc<Mutex<Option<UpstreamResponseCaptureWriter>>>,
    pub(super) diagnostics: DiagnosticsSink,
    pub(super) span: tracing::Span,
}

impl ObserveContext {
    pub(crate) fn new(
        request_id: RequestId,
        started: Instant,
        capture: CaptureSession,
        span: tracing::Span,
    ) -> Self {
        Self {
            request_id,
            started,
            capture,
            stream_capture_writer: Arc::new(Mutex::new(None)),
            diagnostics: DiagnosticsSink::new(request_id),
            span,
        }
    }

    pub(crate) fn start(capture_controller: CaptureController) -> Self {
        let request_id = generate_request_id();
        let span = info_span!("request", request_id = request_id.as_u64());
        let started = Instant::now();
        let capture = capture_controller.session(request_id);
        Self::new(request_id, started, capture, span)
    }

    pub(crate) async fn instrument<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        future.instrument(self.span.clone()).await
    }

    pub(crate) fn elapsed(&self) -> std::time::Duration {
        self.started.elapsed()
    }
}

fn generate_request_id() -> RequestId {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
        .into()
}
