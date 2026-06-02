use axum::body::Bytes;
use futures_util::{Future, Stream, StreamExt};

use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use tokio::time::{Instant as TokioInstant, Sleep};

use crate::capture::UpstreamResponseCaptureWriter;

use crate::upstream::{UpstreamBodyStreamStats, UpstreamResponseHead};

const WAIT_LOG_AFTER: Duration = Duration::from_secs(5);
const WAIT_LOG_INTERVAL: Duration = Duration::from_secs(5);

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

pub(crate) struct MonitoredBodyStream<O>
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

impl<O> MonitoredBodyStream<O>
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

impl<O> Stream for MonitoredBodyStream<O>
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

impl<O> Drop for MonitoredBodyStream<O>
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

#[cfg(test)]
#[path = "monitor_tests.rs"]
mod tests;
