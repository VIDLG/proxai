use axum::body::Bytes;

use futures_util::{Future, Stream, StreamExt};

use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use tokio::time::{Instant as TokioInstant, Sleep};

use crate::http_support::{
    ByteStream, ByteStreamError, OutboundResponseHead, UpstreamResponseHead, boxed_stream_error,
};

use crate::observe::{
    ObserveContext, OutboundResponseHeadPrepared, UpstreamStreamChunkReceived,
    UpstreamStreamProgress, UpstreamStreamingResponseStarted,
};
use crate::upstream::UpstreamBodyStreamStats;

const WAIT_LOG_AFTER: Duration = Duration::from_secs(5);
const WAIT_LOG_INTERVAL: Duration = Duration::from_secs(5);

pub(crate) enum BodyAction {
    Continue,
    InjectAndClose(Bytes),
}

pub(crate) trait BodyObserver: Send + Unpin + 'static {
    fn observe_chunk(&mut self, _chunk: &[u8]) -> BodyAction {
        BodyAction::Continue
    }
    fn observe_error(&mut self, error: &reqwest::Error);
    fn poll_pending(&mut self, _cx: &mut Context<'_>) -> BodyAction {
        BodyAction::Continue
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
    body_observer: O,
    stream: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>,
    obs: ObserveContext,
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
        body_observer: O,
        obs: ObserveContext,
        idle_timeout: Option<Duration>,
    ) -> Self {
        let now = TokioInstant::now();
        Self {
            upstream_head,
            stats: UpstreamBodyStreamStats::new(started),
            finished: false,
            body_observer,
            stream: Box::pin(stream),
            obs,
            last_activity: now,
            wait_sleep: Box::pin(tokio::time::sleep_until(now + WAIT_LOG_AFTER)),
            idle_timeout,
        }
    }

    fn record_chunk(&mut self, chunk: &Bytes) -> BodyAction {
        self.reset_wait_progress();
        self.stats.record_chunk(chunk);
        self.obs
            .observe_upstream_stream_chunk(UpstreamStreamChunkReceived { chunk });
        self.body_observer.observe_chunk(chunk)
    }

    fn emit_outcome(&self) {
        self.body_observer
            .emit_outcome(&self.upstream_head, self.stats);
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
        let progress = UpstreamStreamProgress {
            idle_ms,
            duration_ms,
            chunks,
            down,
        };
        self.obs.observe_upstream_stream_wait(progress);

        let timed_out = self
            .idle_timeout
            .is_some_and(|timeout| idle_ms >= timeout.as_millis() as u64);

        if timed_out {
            self.obs.observe_upstream_stream_timeout(progress);
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
    type Item = Result<Bytes, ByteStreamError>;

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
                this.body_observer.observe_error(&error);
                this.emit_outcome();
                Poll::Ready(Some(Err(boxed_stream_error(error))))
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
                match this.body_observer.poll_pending(cx) {
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
pub(crate) fn prepare_response_stream<O>(
    obs: &ObserveContext,
    head: &UpstreamResponseHead,
    read_idle_timeout: Duration,
    upstream_response: reqwest::Response,
    body_observer: O,
) -> (OutboundResponseHead, ByteStream)
where
    O: BodyObserver,
{
    obs.observe_upstream_streaming_success(UpstreamStreamingResponseStarted { head });
    let outbound_head = OutboundResponseHead::from_upstream(head);
    obs.observe_outbound_response_head_prepared(OutboundResponseHeadPrepared {
        head: &outbound_head,
    });
    let stream = MonitoredUpstreamBodyStream::new(
        upstream_response.bytes_stream(),
        head.clone(),
        obs.started(),
        body_observer,
        obs.clone(),
        Some(read_idle_timeout),
    );

    (outbound_head, Box::pin(stream))
}

#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
