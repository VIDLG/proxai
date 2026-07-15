use serde_json::Value;

use crate::json::deserialize_value;
use crate::protocol::RequestProtocol;
use crate::protocol::openai::chat_completions::CreateChatCompletionRequest;

use super::{IngressError, PreparedInboundRequest};

pub(super) fn prepare_openai_chat_completions_request(
    payload: Value,
) -> Result<PreparedInboundRequest, IngressError> {
    let parsed = deserialize_value::<CreateChatCompletionRequest>(
        &payload,
        "OpenAI Chat Completions request payload",
    )?;
    if parsed.model.trim().is_empty() {
        return Err(IngressError::MissingModel {
            protocol: RequestProtocol::OpenaiChatCompletions,
        });
    }

    Ok(PreparedInboundRequest::new(
        RequestProtocol::OpenaiChatCompletions,
        payload,
        parsed.model,
    ))
}

#[cfg(test)]
mod tests;
