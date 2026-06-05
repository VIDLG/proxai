use axum::body::Bytes;
use axum::http::{HeaderMap, Method, Uri};

use crate::error::Error;
use crate::http_support::{OutboundResponseHead, UpstreamResponseHead};
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::provider::ProviderRequestView;
use crate::provider::anthropic_messages::AnthropicUpstreamResponseSnapshot;
use crate::provider::openai::chat_completions::ChatUpstreamStreamSnapshot;
use crate::provider::openai::responses::ResponsesUpstreamStreamSnapshot;
use crate::upstream::UpstreamStreamError;

pub(crate) struct InboundRequestReceived<'a> {
    pub(crate) method: &'a Method,
    pub(crate) uri: &'a Uri,
    pub(crate) headers: &'a HeaderMap,
}

pub(crate) struct InboundRequestPrepared<'a> {
    pub(crate) method: &'a Method,
    pub(crate) uri: &'a Uri,
    pub(crate) headers: &'a HeaderMap,
    pub(crate) body: &'a Bytes,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProviderRequestBodySizes {
    pub(crate) inbound: usize,
    pub(crate) provider: usize,
}

impl ProviderRequestBodySizes {
    pub(crate) fn delta(self) -> i128 {
        self.provider as i128 - self.inbound as i128
    }
}

pub(crate) struct RequestFailed<'a> {
    pub(crate) error: &'a Error,
}

pub(crate) struct ProviderProtocolRequestPrepared<'a> {
    pub(crate) method: Method,
    pub(crate) uri: Uri,
    pub(crate) request_sizes: ProviderRequestBodySizes,
    pub(crate) request_protocol: RequestProtocol,
    pub(crate) provider: String,
    pub(crate) route_name: Option<String>,
    pub(crate) provider_protocol: ProviderProtocol,
    pub(crate) provider_request: ProviderRequestView<'a>,
}

pub(crate) struct RequestInfoParseFailure<'a> {
    pub(crate) normalized_payload: &'a serde_json::Value,
    pub(crate) request_info_parse_payload: &'a serde_json::Value,
    pub(crate) error: &'a serde_json::Error,
}

pub(crate) struct ProviderHttpRequestPrepared<'a> {
    pub(crate) method: &'a Method,
    pub(crate) url: &'a str,
    pub(crate) headers: &'a HeaderMap,
    pub(crate) body: &'a [u8],
    pub(crate) normalized_payload: Option<&'a serde_json::Value>,
}

pub(crate) struct UpstreamResponseHeadReceived<'a> {
    pub(crate) head: &'a UpstreamResponseHead,
}

pub(crate) struct OutboundResponseHeadPrepared<'a> {
    pub(crate) head: &'a OutboundResponseHead,
}

pub(crate) struct UpstreamNonStreamingResponseReceived<'a> {
    pub(crate) head: &'a UpstreamResponseHead,
    pub(crate) body: &'a [u8],
}

pub(crate) struct UpstreamStreamingResponseStarted<'a> {
    pub(crate) head: &'a UpstreamResponseHead,
}

pub(crate) struct UpstreamErrorResponseReceived<'a> {
    pub(crate) head: &'a UpstreamResponseHead,
    pub(crate) body: &'a [u8],
}

pub(crate) struct UpstreamStreamChunkReceived<'a> {
    pub(crate) chunk: &'a [u8],
}

pub(crate) enum ProviderStreamSnapshot<'a> {
    AnthropicMessages(&'a AnthropicUpstreamResponseSnapshot),
    OpenaiChatCompletions(&'a ChatUpstreamStreamSnapshot),
    OpenaiResponses(&'a ResponsesUpstreamStreamSnapshot),
}

pub(crate) enum ProviderStreamOutcome<'a> {
    Completed,
    Closed,
    Error(&'a UpstreamStreamError),
    UnfinishedTool(&'a UpstreamStreamError),
}

pub(crate) struct ProviderStreamOutcomeObserved<'a> {
    pub(crate) snapshot: ProviderStreamSnapshot<'a>,
    pub(crate) outcome: ProviderStreamOutcome<'a>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct UpstreamStreamProgress {
    pub(crate) idle_ms: u64,
    pub(crate) duration_ms: u64,
    pub(crate) chunks: u64,
    pub(crate) down: u64,
}
