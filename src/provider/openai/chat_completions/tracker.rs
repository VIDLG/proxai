use async_openai::types::chat as openai;
use axum::http::HeaderMap;

use crate::protocol::openai::chat_completions::{
    ChatStreamResponseProjection, CreateChatCompletionStreamResponse,
};
use crate::sse::SseEventScanner;
use crate::upstream::UpstreamResponseHead;
use std::time::Duration;

use super::observed::ObservedChatResponse;
use super::state::ChatUpstreamResponseState;

#[derive(Debug, Default)]
pub(crate) struct ChatUpstreamResponseTracker {
    pub(crate) state: ChatUpstreamResponseState,
    sse_scanner: SseEventScanner,
    is_sse: bool,
    json_body: Vec<u8>,
}

impl ChatUpstreamResponseTracker {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        let head = UpstreamResponseHead::from_headers(
            axum::http::StatusCode::OK,
            headers,
            Duration::default(),
        );
        Self {
            is_sse: head.is_sse(),
            state: ChatUpstreamResponseState {
                is_sse: head.is_sse(),
                ..ChatUpstreamResponseState::default()
            },
            ..Self::default()
        }
    }

    pub(crate) fn scan_bytes(&mut self, chunk: &[u8]) {
        if self.is_sse {
            self.scan_sse_bytes(chunk);
        } else {
            self.json_body.extend_from_slice(chunk);
        }
    }

    #[cfg(test)]
    pub(crate) fn finish(&mut self) {
        use crate::protocol::openai::chat_completions::{
            ChatResponseProjection, CreateChatCompletionResponse,
        };

        if self.is_sse || self.json_body.is_empty() {
            return;
        }
        let Ok(response) =
            serde_json::from_slice::<openai::CreateChatCompletionResponse>(&self.json_body)
        else {
            return;
        };
        let projection = ChatResponseProjection::from(CreateChatCompletionResponse::from(response));
        self.state
            .observed
            .record(ObservedChatResponse::Response(projection));
        self.json_body.clear();
    }

    fn scan_sse_bytes(&mut self, chunk: &[u8]) {
        for event in self.sse_scanner.scan(chunk) {
            if event.is_done_sentinel() {
                self.state.stream_done = true;
                continue;
            }
            let Ok(response) =
                serde_json::from_str::<openai::CreateChatCompletionStreamResponse>(&event.data)
            else {
                continue;
            };
            let projection = ChatStreamResponseProjection::from(
                CreateChatCompletionStreamResponse::from(response),
            );
            self.state
                .observed
                .record(ObservedChatResponse::Stream(projection));
        }
    }
}
