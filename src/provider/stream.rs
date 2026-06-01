use axum::body::Bytes;
use futures_util::{Stream, StreamExt};

use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;

use crate::capture::UpstreamResponseCaptureWriter;

use crate::upstream::{UpstreamResponseHead, UpstreamStreamMetrics};

#[derive(Debug, Clone, Copy)]
pub(crate) struct UpstreamBodyStreamStats {
    started: Instant,
    chunks: u64,
    bytes: u64,
}

impl UpstreamBodyStreamStats {
    fn new(started: Instant) -> Self {
        Self {
            started,
            chunks: 0,
            bytes: 0,
        }
    }

    fn record_chunk(&mut self, chunk: &[u8]) {
        self.chunks += 1;
        self.bytes += chunk.len() as u64;
    }

    pub(crate) fn metrics(&self) -> UpstreamStreamMetrics {
        UpstreamStreamMetrics::new(self.started.elapsed(), self.chunks, self.bytes)
    }
}

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
    ) -> Self {
        Self {
            upstream_head,
            stats: UpstreamBodyStreamStats::new(started),
            finished: false,
            observer,
            capture_writer,
            stream: Box::pin(stream),
        }
    }

    fn record_chunk(&mut self, chunk: &Bytes) -> BodyAction {
        self.stats.record_chunk(chunk);
        if let Some(writer) = self.capture_writer.as_mut() {
            writer.write_chunk(chunk);
        }
        self.observer.observe_chunk(chunk)
    }

    fn emit_outcome(&self) {
        self.observer.emit_outcome(&self.upstream_head, self.stats);
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
            Poll::Pending => match this.observer.poll_pending(cx) {
                BodyAction::Continue => Poll::Pending,
                BodyAction::InjectAndClose(chunk) => {
                    this.finished = true;
                    this.record_chunk(&chunk);
                    this.emit_outcome();
                    Poll::Ready(Some(Ok(chunk)))
                }
            },
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
#[path = "stream_tests.rs"]
mod tests;
