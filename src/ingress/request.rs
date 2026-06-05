use serde_json::Value;

use crate::error::{RequestError, Result};
use crate::protocol::RequestProtocol;

#[derive(Debug)]
pub(crate) enum PreparedInboundRequest {
    OpenaiResponses(super::openai_responses::PreparedOpenaiResponsesRequest),
    OpenaiChatCompletions(super::openai_chat_completions::PreparedOpenaiChatCompletionsRequest),
    AnthropicMessages(super::anthropic_messages::PreparedAnthropicMessagesRequest),
}

impl PreparedInboundRequest {
    pub(crate) fn protocol(&self) -> RequestProtocol {
        match self {
            Self::OpenaiResponses(_) => RequestProtocol::OpenaiResponses,
            Self::OpenaiChatCompletions(_) => RequestProtocol::OpenaiChatCompletions,
            Self::AnthropicMessages(_) => RequestProtocol::AnthropicMessages,
        }
    }

    pub(crate) fn model(&self) -> &str {
        match self {
            Self::OpenaiResponses(request) => &request.model,
            Self::OpenaiChatCompletions(request) => &request.model,
            Self::AnthropicMessages(request) => &request.model,
        }
    }

    pub(crate) fn normalized_payload(&self) -> &Value {
        match self {
            Self::OpenaiResponses(request) => &request.normalized_payload,
            Self::OpenaiChatCompletions(request) => &request.normalized_payload,
            Self::AnthropicMessages(request) => &request.normalized_payload,
        }
    }

    pub(crate) fn body_len(&self) -> usize {
        self.normalized_payload().to_string().len()
    }
}

pub(crate) fn prepare_inbound_request(
    protocol: RequestProtocol,
    body: &[u8],
) -> Result<PreparedInboundRequest, RequestError> {
    match protocol {
        RequestProtocol::OpenaiResponses => Ok(PreparedInboundRequest::OpenaiResponses(
            super::openai_responses::prepare_openai_responses_request(body)?,
        )),
        RequestProtocol::OpenaiChatCompletions => {
            Ok(PreparedInboundRequest::OpenaiChatCompletions(
                super::openai_chat_completions::prepare_openai_chat_completions_request(body)?,
            ))
        }
        RequestProtocol::AnthropicMessages => Ok(PreparedInboundRequest::AnthropicMessages(
            super::anthropic_messages::prepare_anthropic_messages_request(body)?,
        )),
    }
}
