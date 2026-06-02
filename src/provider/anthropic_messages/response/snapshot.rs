use std::time::Duration;

use crate::upstream::{UpstreamBodyStreamStats, UpstreamResponseHead, UpstreamStreamMetrics};

use super::state::AnthropicResponseState;

#[derive(Debug, Clone)]
pub(crate) struct AnthropicUpstreamResponseSnapshot {
    pub(crate) head: UpstreamResponseHead,
    pub(crate) metrics: UpstreamStreamMetrics,
    pub(crate) state: AnthropicResponseState,
}

impl AnthropicUpstreamResponseSnapshot {
    pub(crate) fn non_streaming(
        upstream_head: &UpstreamResponseHead,
        body_len: usize,
        duration: Duration,
        state: AnthropicResponseState,
    ) -> Self {
        let bytes = body_len as u64;
        let mut head = upstream_head.clone();
        head.content_length = Some(bytes);
        head.transfer_encoding = None;

        Self {
            head,
            metrics: UpstreamStreamMetrics::new(duration, 1, bytes),
            state,
        }
    }

    pub(crate) fn streaming(
        head: &UpstreamResponseHead,
        stats: UpstreamBodyStreamStats,
        state: AnthropicResponseState,
    ) -> Self {
        Self {
            head: head.clone(),
            metrics: stats.metrics(),
            state,
        }
    }
}
