use serde_json::Value;

use crate::error::{InternalError, Result};
use crate::observe::ObserveContext;
use crate::protocol::ProviderProtocol;

use crate::protocol::anthropic::messages::MessageCreateParamsBase;
use crate::protocol::openai::{chat_completions, responses as openai_responses};
use crate::provider::anthropic_messages as anthropic_provider;
use crate::provider::openai::{chat_completions as chat_provider, responses as responses_provider};

pub(crate) fn assemble_request(
    protocol: ProviderProtocol,
    provider_payload: Value,
    obs: &ObserveContext,
) -> Result<ProviderRequest, InternalError> {
    match protocol {
        ProviderProtocol::OpenaiResponses => {
            let body = serde_json::to_vec(&provider_payload)?;
            let prepared = responses_provider::request::prepare_provider_request(
                &provider_payload,
                body,
                obs,
            )?;
            Ok(ProviderRequest::openai_responses(
                prepared,
                provider_payload,
            ))
        }
        ProviderProtocol::OpenaiChatCompletions => {
            let body = serde_json::to_vec(&provider_payload)?;
            let prepared =
                chat_provider::request::prepare_provider_request(&provider_payload, body)?;
            Ok(ProviderRequest::openai_chat_completions(
                prepared,
                provider_payload,
            ))
        }
        ProviderProtocol::AnthropicMessages => {
            let body = serde_json::to_vec(&provider_payload)?;
            let prepared =
                anthropic_provider::request::prepare_provider_request(&provider_payload, body)?;
            Ok(ProviderRequest::anthropic_messages(
                prepared,
                provider_payload,
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ProviderRequest {
    prepared: PreparedProviderRequest,
    capture_payload: Value,
}

#[derive(Debug, Clone)]
pub(crate) enum PreparedProviderRequest {
    OpenaiResponses(Box<responses_provider::request::PreparedProviderRequest>),
    OpenaiChatCompletions(Box<chat_provider::request::PreparedProviderRequest>),
    AnthropicMessages(Box<anthropic_provider::request::PreparedProviderRequest>),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ProviderRequestView<'a> {
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

impl ProviderRequest {
    pub(crate) fn anthropic_messages(
        prepared: anthropic_provider::request::PreparedProviderRequest,
        capture_payload: Value,
    ) -> Self {
        Self {
            prepared: PreparedProviderRequest::AnthropicMessages(Box::new(prepared)),
            capture_payload,
        }
    }

    pub(crate) fn openai_responses(
        prepared: responses_provider::request::PreparedProviderRequest,
        capture_payload: Value,
    ) -> Self {
        Self {
            prepared: PreparedProviderRequest::OpenaiResponses(Box::new(prepared)),
            capture_payload,
        }
    }

    pub(crate) fn openai_chat_completions(
        prepared: chat_provider::request::PreparedProviderRequest,
        capture_payload: Value,
    ) -> Self {
        Self {
            prepared: PreparedProviderRequest::OpenaiChatCompletions(Box::new(prepared)),
            capture_payload,
        }
    }

    pub(crate) fn body(&self) -> &[u8] {
        match &self.prepared {
            PreparedProviderRequest::OpenaiResponses(request) => &request.body,
            PreparedProviderRequest::OpenaiChatCompletions(request) => &request.body,
            PreparedProviderRequest::AnthropicMessages(request) => &request.body,
        }
    }

    pub(crate) fn upstream_path(&self) -> &'static str {
        match &self.prepared {
            PreparedProviderRequest::OpenaiResponses(_) => {
                responses_provider::request::UPSTREAM_PATH
            }
            PreparedProviderRequest::OpenaiChatCompletions(_) => {
                chat_provider::request::UPSTREAM_PATH
            }
            PreparedProviderRequest::AnthropicMessages(_) => {
                anthropic_provider::request::UPSTREAM_PATH
            }
        }
    }

    pub(crate) fn into_body(self) -> Vec<u8> {
        match self.prepared {
            PreparedProviderRequest::OpenaiResponses(request) => request.body,
            PreparedProviderRequest::OpenaiChatCompletions(request) => request.body,
            PreparedProviderRequest::AnthropicMessages(request) => request.body,
        }
    }

    pub(crate) fn capture_payload(&self) -> &Value {
        &self.capture_payload
    }

    pub(crate) fn view(&self) -> ProviderRequestView<'_> {
        match &self.prepared {
            PreparedProviderRequest::OpenaiResponses(request) => {
                ProviderRequestView::OpenaiResponses {
                    projection: &request.projection,
                    summary: &request.summary,
                }
            }
            PreparedProviderRequest::OpenaiChatCompletions(request) => {
                ProviderRequestView::OpenaiChatCompletions {
                    projection: &request.projection,
                    summary: &request.summary,
                }
            }
            PreparedProviderRequest::AnthropicMessages(request) => {
                ProviderRequestView::AnthropicMessages {
                    projection: &request.projection,
                    summary: &request.summary,
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "request_regression_tests.rs"]
mod regression_tests;
