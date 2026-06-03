use axum::body::Bytes;
use axum::http::HeaderMap;
use futures_util::{Future, Stream, StreamExt};

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use tokio::time::{Instant as TokioInstant, Sleep};

use crate::capture::{CaptureSession, UpstreamResponseCaptureWriter};

use crate::http_model::UpstreamResponseHead;
use crate::http_utils::filter_forwardable_headers;

use crate::upstream::UpstreamBodyStreamStats;

const WAIT_LOG_AFTER: Duration = Duration::from_secs(5);
const WAIT_LOG_INTERVAL: Duration = Duration::from_secs(5);

pub(crate) struct StreamingResponseContext<'a> {
    pub(crate) capture: &'a CaptureSession,
    pub(crate) started: Instant,
    pub(crate) span: &'a tracing::Span,
    pub(crate) read_idle_timeout: Duration,
    pub(crate) head: &'a UpstreamResponseHead,
}

pub(crate) enum BodyAction {
    Continue,
    InjectAndClose(Bytes),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ProgressFields {
    pub(crate) phase: &'static str,
    pub(crate) response_id: Option<String>,
    pub(crate) sequence_number: Option<u64>,
    pub(crate) response_status: Option<String>,
    pub(crate) snapshot_kind: Option<String>,
    pub(crate) pending_tool_items: Option<u64>,
}

pub(crate) trait BodyObserver: Send + Unpin + 'static {
    fn observe_chunk(&mut self, _chunk: &[u8]) -> BodyAction {
        BodyAction::Continue
    }
    fn observe_error(&mut self, error: &reqwest::Error);
    fn poll_pending(&mut self, _cx: &mut Context<'_>) -> BodyAction {
        BodyAction::Continue
    }
    fn progress_fields(&self) -> ProgressFields {
        ProgressFields {
            phase: "upstream",
            ..ProgressFields::default()
        }
    }
    fn emit_outcome(&self, head: &UpstreamResponseHead, stats: UpstreamBodyStreamStats);
}

pub(crate) struct MonitoredUpstreamBodyStream<O>
where
    O: BodyObserver,
{
    upstream_head: UpstreamResponseHead,
    stats: UpstreamBodyStreamStats,
    finished: bool,
    observer: O,
    capture_writer: Option<UpstreamResponseCaptureWriter>,
    stream: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>,
    span: tracing::Span,
    last_activity: TokioInstant,
    wait_sleep: Pin<Box<Sleep>>,
    idle_timeout: Option<Duration>,
}

impl<O> MonitoredUpstreamBodyStream<O>
where
    O: BodyObserver,
{
    pub(crate) fn new(
        stream: impl Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
        upstream_head: UpstreamResponseHead,
        started: Instant,
        observer: O,
        capture_writer: Option<UpstreamResponseCaptureWriter>,
        span: tracing::Span,
        idle_timeout: Option<Duration>,
    ) -> Self {
        let now = TokioInstant::now();
        Self {
            upstream_head,
            stats: UpstreamBodyStreamStats::new(started),
            finished: false,
            observer,
            capture_writer,
            stream: Box::pin(stream),
            span,
            last_activity: now,
            wait_sleep: Box::pin(tokio::time::sleep_until(now + WAIT_LOG_AFTER)),
            idle_timeout,
        }
    }

    fn record_chunk(&mut self, chunk: &Bytes) -> BodyAction {
        self.reset_wait_progress();
        self.stats.record_chunk(chunk);
        if let Some(writer) = self.capture_writer.as_mut() {
            writer.write_chunk(chunk);
        }
        self.observer.observe_chunk(chunk)
    }

    fn emit_outcome(&self) {
        self.observer.emit_outcome(&self.upstream_head, self.stats);
    }

    fn reset_wait_progress(&mut self) {
        let now = TokioInstant::now();
        self.last_activity = now;
        self.wait_sleep.as_mut().reset(now + WAIT_LOG_AFTER);
    }

    fn poll_wait_progress(&mut self, cx: &mut Context<'_>) -> bool {
        if self.wait_sleep.as_mut().poll(cx).is_pending() {
            return false;
        }

        let idle_ms = self.last_activity.elapsed().as_millis() as u64;
        let duration_ms = self.stats.metrics().duration_ms();
        let chunks = self.stats.chunks();
        let down = self.stats.bytes();
        let progress = self.observer.progress_fields();
        self.span.in_scope(|| {
            tracing::info!(
                event = "wait",
                phase = progress.phase,
                idle_ms,
                duration_ms,
                chunks,
                down,
                response_id = progress.response_id,
                seq = progress.sequence_number,
                response_status = progress.response_status,
                snapshot_kind = progress.snapshot_kind,
                pending_tool_items = progress.pending_tool_items,
            );
        });

        let timed_out = self
            .idle_timeout
            .is_some_and(|timeout| idle_ms >= timeout.as_millis() as u64);

        if timed_out {
            self.span.in_scope(|| {
                tracing::warn!(
                    event = "timeout",
                    idle_ms,
                    duration_ms,
                    chunks,
                    down,
                    "stream idle timeout exceeded"
                );
            });
        } else {
            self.wait_sleep
                .as_mut()
                .reset(TokioInstant::now() + WAIT_LOG_INTERVAL);
        }

        timed_out
    }
}

impl<O> Stream for MonitoredUpstreamBodyStream<O>
where
    O: BodyObserver,
{
    type Item = std::io::Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.finished {
            return Poll::Ready(None);
        }

        match this.stream.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(chunk))) => match this.record_chunk(&chunk) {
                BodyAction::Continue => Poll::Ready(Some(Ok(chunk))),
                BodyAction::InjectAndClose(chunk) => {
                    this.finished = true;
                    this.emit_outcome();
                    Poll::Ready(Some(Ok(chunk)))
                }
            },
            Poll::Ready(Some(Err(error))) => {
                this.finished = true;
                this.observer.observe_error(&error);
                this.emit_outcome();
                Poll::Ready(Some(Err(std::io::Error::other(error))))
            }
            Poll::Ready(None) => {
                this.finished = true;
                this.emit_outcome();
                Poll::Ready(None)
            }
            Poll::Pending => {
                let timed_out = this.poll_wait_progress(cx);
                if timed_out {
                    this.finished = true;
                    this.emit_outcome();
                    return Poll::Ready(None);
                }
                match this.observer.poll_pending(cx) {
                    BodyAction::Continue => Poll::Pending,
                    BodyAction::InjectAndClose(chunk) => {
                        this.finished = true;
                        this.record_chunk(&chunk);
                        this.emit_outcome();
                        Poll::Ready(Some(Ok(chunk)))
                    }
                }
            }
        }
    }
}

impl<O> Drop for MonitoredUpstreamBodyStream<O>
where
    O: BodyObserver,
{
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        self.emit_outcome();
    }
}

/// Prepare the streaming body side of a 2xx upstream response.
///
/// This consolidates the steps shared by streaming provider response handlers:
///
/// 1. Capture the raw upstream headers to the configured capture destination.
/// 2. Filter the upstream headers down to the set safe to forward to the client.
/// 3. Capture the outbound headers so the on-disk capture mirrors what the
///    client actually saw.
/// 4. Wrap the reqwest byte stream in a [`MonitoredUpstreamBodyStream`] driven by the
///    caller-supplied [`BodyObserver`].
pub(crate) async fn prepare_response_stream<O>(
    context: StreamingResponseContext<'_>,
    upstream_response: reqwest::Response,
    observer: O,
) -> (
    HeaderMap,
    Pin<Box<dyn Stream<Item = io::Result<Bytes>> + Send>>,
)
where
    O: BodyObserver,
{
    context
        .capture
        .capture_upstream_response_headers(context.head)
        .await;

    let outbound_headers = filter_forwardable_headers(&context.head.headers);
    context
        .capture
        .capture_outbound_response_headers(
            context.head.status,
            context.head.content_type().as_ref().map(AsRef::as_ref),
            &outbound_headers,
        )
        .await;

    let capture_writer = context
        .capture
        .create_upstream_response_writer(context.head.content_type().as_ref());
    let stream = MonitoredUpstreamBodyStream::new(
        upstream_response.bytes_stream(),
        context.head.clone(),
        context.started,
        observer,
        capture_writer,
        context.span.clone(),
        Some(context.read_idle_timeout),
    );

    (outbound_headers, Box::pin(stream))
}

#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
