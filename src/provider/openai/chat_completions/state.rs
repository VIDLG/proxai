use crate::upstream::{UpstreamResponseHead, UpstreamStreamMetrics};

use super::observed::ObservedChatResponseState;

#[derive(Debug, Clone, Default)]
pub(crate) struct ChatUpstreamResponseState {
    pub(crate) observed: ObservedChatResponseState,
    pub(crate) stream_done: bool,
    pub(crate) is_sse: bool,
}

impl ChatUpstreamResponseState {
    pub(crate) fn eof_is_complete(&self) -> bool {
        !self.is_sse || self.stream_done
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ChatUpstreamStreamSnapshot {
    pub(crate) head: UpstreamResponseHead,
    pub(crate) metrics: UpstreamStreamMetrics,
    pub(crate) state: ChatUpstreamResponseState,
}
