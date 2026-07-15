use getset::{CopyGetters, Getters};
use serde_json::Value;

use crate::observe::Observer;
use crate::protocol::RequestProtocol;

use super::IngressResult;

#[derive(Debug, Getters, CopyGetters)]
pub struct PreparedInboundRequest {
    #[getset(get_copy = "pub")]
    protocol: RequestProtocol,
    #[getset(get = "pub")]
    normalized_payload: Value,
    #[getset(get = "pub")]
    model: String,
}

impl PreparedInboundRequest {
    pub(super) fn new(protocol: RequestProtocol, normalized_payload: Value, model: String) -> Self {
        Self {
            protocol,
            normalized_payload,
            model,
        }
    }

    pub fn normalized_payload_len(&self) -> usize {
        self.normalized_payload.to_string().len()
    }
}

pub fn prepare_inbound_request(
    protocol: RequestProtocol,
    payload: Value,
    observer: &dyn Observer,
) -> IngressResult<PreparedInboundRequest> {
    match protocol {
        RequestProtocol::OpenaiResponses => {
            super::openai_responses::prepare_openai_responses_request(payload)
        }
        RequestProtocol::OpenaiChatCompletions => {
            super::openai_chat_completions::prepare_openai_chat_completions_request(payload)
        }
        RequestProtocol::AnthropicMessages => {
            super::anthropic_messages::prepare_anthropic_messages_request(payload, observer)
        }
    }
}
