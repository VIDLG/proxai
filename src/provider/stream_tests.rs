use axum::body::Bytes;
use futures_util::{stream, StreamExt};
use std::sync::{Arc, Mutex};
use std::task::Context;
use std::time::Instant;

use super::{BodyAction, BodyObserver, MonitoredBodyStream, UpstreamBodyStreamStats};
use crate::upstream::UpstreamResponseHead;

#[derive(Default)]
struct ObserverState {
    chunks: usize,
    errored: bool,
    outcomes: usize,
    outcome_chunks: u64,
    outcome_bytes: u64,
}

#[derive(Clone)]
struct TestObserver {
    state: Arc<Mutex<ObserverState>>,
    inject_on_pending: Option<Bytes>,
}

impl Default for TestObserver {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(ObserverState::default())),
            inject_on_pending: None,
        }
    }
}

impl BodyObserver for TestObserver {
    fn observe_chunk(&mut self, _chunk: &[u8]) -> BodyAction {
        self.state.lock().unwrap().chunks += 1;
        BodyAction::Continue
    }

    fn observe_error(&mut self, _error: &reqwest::Error) {
        self.state.lock().unwrap().errored = true;
    }

    fn poll_pending(&mut self, _cx: &mut Context<'_>) -> BodyAction {
        self.inject_on_pending
            .take()
            .map_or(BodyAction::Continue, BodyAction::InjectAndClose)
    }

    fn emit_outcome(&self, _head: &UpstreamResponseHead, stats: UpstreamBodyStreamStats) {
        let metrics = stats.metrics();
        let mut state = self.state.lock().unwrap();
        state.outcomes += 1;
        state.outcome_chunks = metrics.chunks;
        state.outcome_bytes = metrics.bytes;
    }
}

fn test_head() -> UpstreamResponseHead {
    UpstreamResponseHead::from_headers(
        http::StatusCode::OK,
        &http::HeaderMap::new(),
        Default::default(),
    )
}

#[tokio::test]
async fn generic_stream_records_chunks_and_outcome() {
    let observer = TestObserver::default();
    let state = observer.state.clone();
    let stream = MonitoredBodyStream::new(
        stream::iter([
            Ok::<_, reqwest::Error>(Bytes::from_static(b"a")),
            Ok(Bytes::from_static(b"b")),
        ]),
        test_head(),
        Instant::now(),
        observer,
        None,
    );

    let body = stream
        .map(|chunk| chunk.unwrap())
        .collect::<Vec<_>>()
        .await
        .concat();

    assert_eq!(body, b"ab");
    let state = state.lock().unwrap();
    assert_eq!(state.chunks, 2);
    assert!(!state.errored);
    assert_eq!(state.outcomes, 1);
    assert_eq!(state.outcome_chunks, 2);
    assert_eq!(state.outcome_bytes, 2);
}

#[tokio::test]
async fn generic_stream_allows_observer_inject_and_close_on_pending() {
    let observer = TestObserver {
        inject_on_pending: Some(Bytes::from_static(b"timeout")),
        ..TestObserver::default()
    };
    let state = observer.state.clone();
    let stream = MonitoredBodyStream::new(
        futures_util::stream::pending::<reqwest::Result<Bytes>>(),
        test_head(),
        Instant::now(),
        observer,
        None,
    );

    let body = stream
        .take(1)
        .map(|chunk| chunk.unwrap())
        .collect::<Vec<_>>()
        .await
        .concat();

    assert_eq!(body, b"timeout");
    let state = state.lock().unwrap();
    assert_eq!(state.chunks, 1);
    assert_eq!(state.outcomes, 1);
    assert_eq!(state.outcome_chunks, 1);
    assert_eq!(state.outcome_bytes, 7);
}
