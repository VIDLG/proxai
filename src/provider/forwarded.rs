use serde_json::Value;

use crate::protocol::anthropic::messages::MessageCreateParamsBase;
use crate::protocol::openai::{chat_completions, responses as openai_responses};
use crate::provider::anthropic_messages as anthropic_provider;
use crate::provider::openai::{chat_completions as chat_provider, responses as responses_provider};

#[derive(Debug, Clone)]
pub(crate) struct ForwardedRequest {
    prepared: PreparedForwardedRequest,
    capture_payload: Value,
}

#[derive(Debug, Clone)]
pub(crate) enum PreparedForwardedRequest {
    OpenaiResponses(Box<responses_provider::request::PreparedForwardedRequest>),
    OpenaiChatCompletions(Box<chat_provider::request::PreparedForwardedRequest>),
    AnthropicMessages(Box<anthropic_provider::request::PreparedForwardedRequest>),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ForwardedRequestView<'a> {
    OpenaiResponses {
        projection: &'a openai_responses::RequestProjection,
        summary: &'a responses_provider::RequestSummary,
    },
    OpenaiChatCompletions {
        projection: &'a chat_completions::RequestProjection,
        summary: &'a chat_provider::RequestSummary,
    },
    AnthropicMessages {
        projection: &'a MessageCreateParamsBase,
        summary: &'a anthropic_provider::request::RequestSummary,
    },
}

impl ForwardedRequest {
    pub(crate) fn anthropic_messages(
        prepared: anthropic_provider::request::PreparedForwardedRequest,
        capture_payload: Value,
    ) -> Self {
        Self {
            prepared: PreparedForwardedRequest::AnthropicMessages(Box::new(prepared)),
            capture_payload,
        }
    }

    pub(crate) fn openai_responses(
        prepared: responses_provider::request::PreparedForwardedRequest,
        capture_payload: Value,
    ) -> Self {
        Self {
            prepared: PreparedForwardedRequest::OpenaiResponses(Box::new(prepared)),
            capture_payload,
        }
    }

    pub(crate) fn openai_chat_completions(
        prepared: chat_provider::request::PreparedForwardedRequest,
        capture_payload: Value,
    ) -> Self {
        Self {
            prepared: PreparedForwardedRequest::OpenaiChatCompletions(Box::new(prepared)),
            capture_payload,
        }
    }

    pub(crate) fn body(&self) -> &[u8] {
        match &self.prepared {
            PreparedForwardedRequest::OpenaiResponses(request) => &request.body,
            PreparedForwardedRequest::OpenaiChatCompletions(request) => &request.body,
            PreparedForwardedRequest::AnthropicMessages(request) => &request.body,
        }
    }

    pub(crate) fn upstream_path(&self) -> &'static str {
        match &self.prepared {
            PreparedForwardedRequest::OpenaiResponses(_) => {
                responses_provider::request::UPSTREAM_PATH
            }
            PreparedForwardedRequest::OpenaiChatCompletions(_) => {
                chat_provider::request::UPSTREAM_PATH
            }
            PreparedForwardedRequest::AnthropicMessages(_) => {
                anthropic_provider::request::UPSTREAM_PATH
            }
        }
    }

    pub(crate) fn into_body(self) -> Vec<u8> {
        match self.prepared {
            PreparedForwardedRequest::OpenaiResponses(request) => request.body,
            PreparedForwardedRequest::OpenaiChatCompletions(request) => request.body,
            PreparedForwardedRequest::AnthropicMessages(request) => request.body,
        }
    }

    pub(crate) fn capture_payload(&self) -> &Value {
        &self.capture_payload
    }

    pub(crate) fn view(&self) -> ForwardedRequestView<'_> {
        match &self.prepared {
            PreparedForwardedRequest::OpenaiResponses(request) => {
                ForwardedRequestView::OpenaiResponses {
                    projection: &request.projection,
                    summary: &request.summary,
                }
            }
            PreparedForwardedRequest::OpenaiChatCompletions(request) => {
                ForwardedRequestView::OpenaiChatCompletions {
                    projection: &request.projection,
                    summary: &request.summary,
                }
            }
            PreparedForwardedRequest::AnthropicMessages(request) => {
                ForwardedRequestView::AnthropicMessages {
                    projection: &request.projection,
                    summary: &request.summary,
                }
            }
        }
    }
}
