use std::future::Future;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use getset::CopyGetters;
use tracing::{Instrument, info_span};

use super::capture::{CaptureController, CaptureSession};
use super::sinks::ObserveSinks;
use crate::http_support::ByteStream;
use crate::request::RequestId;

#[derive(Clone, CopyGetters)]
pub(crate) struct ObserveContext {
    #[getset(get_copy = "pub(crate)")]
    pub(super) request_id: RequestId,
    #[getset(get_copy = "pub(crate)")]
    pub(super) started: Instant,
    pub(super) sinks: ObserveSinks,
    pub(super) span: tracing::Span,
    failure_reported: Arc<AtomicBool>,
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
            sinks: ObserveSinks::new(request_id, capture),
            span,
            failure_reported: Arc::new(AtomicBool::new(false)),
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

    pub(crate) fn instrument_stream(&self, mut stream: ByteStream) -> ByteStream {
        let span = self.span.clone();
        Box::pin(async_stream::stream! {
            while let Some(item) = stream.as_mut().next().instrument(span.clone()).await {
                yield item;
            }
        })
    }

    pub(super) fn mark_failure_reported(&self) {
        self.failure_reported.store(true, Ordering::Relaxed);
    }

    pub(super) fn should_report_failure(&self) -> bool {
        !self.failure_reported.swap(true, Ordering::Relaxed)
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
