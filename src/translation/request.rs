use crate::error::{InternalError, Result};
use crate::ingress::PreparedInboundRequest;
use crate::protocol::ProviderProtocol;
use crate::provider::anthropic_messages;
use crate::provider::openai::{chat_completions, responses as openai_responses};
use crate::provider::ProviderRequest;

pub(crate) fn translate_request(
    inbound: &PreparedInboundRequest,
    provider_protocol: ProviderProtocol,
    upstream_model: &str,
) -> Result<ProviderRequest, InternalError> {
    match (inbound, provider_protocol) {
        (PreparedInboundRequest::OpenaiResponses(inbound), ProviderProtocol::OpenaiResponses) => {
            Ok(ProviderRequest::openai_responses(
                openai_responses::request::prepare_provider_request(
                    &inbound.normalized_payload,
                    None,
                    &inbound.model,
                    upstream_model,
                )?,
                inbound.normalized_payload.clone(),
            ))
        }
        (
            PreparedInboundRequest::OpenaiChatCompletions(inbound),
            ProviderProtocol::OpenaiChatCompletions,
        ) => Ok(ProviderRequest::openai_chat_completions(
            chat_completions::request::prepare_provider_request(
                &inbound.normalized_payload,
                &inbound.model,
                upstream_model,
            )?,
            inbound.normalized_payload.clone(),
        )),
        (
            PreparedInboundRequest::OpenaiResponses(inbound),
            ProviderProtocol::OpenaiChatCompletions,
        ) => {
            let translated = crate::translation::openai_responses::to_openai_chat_completions::translate_request_payload(
                &inbound.normalized_payload,
                &inbound.model,
                upstream_model,
            )?;
            Ok(ProviderRequest::openai_chat_completions(
                chat_completions::request::prepare_provider_request(
                    &translated,
                    upstream_model,
                    upstream_model,
                )?,
                translated,
            ))
        }
        (PreparedInboundRequest::OpenaiResponses(inbound), ProviderProtocol::AnthropicMessages) => {
            let translated = crate::translation::openai_responses::to_anthropic_messages::translate_request_payload(
                &inbound.normalized_payload,
                &inbound.model,
                upstream_model,
            )?;
            Ok(ProviderRequest::anthropic_messages(
                anthropic_messages::request::prepare_provider_request(
                    &translated,
                    upstream_model,
                    upstream_model,
                )?,
                translated,
            ))
        }
        (PreparedInboundRequest::OpenaiChatCompletions(_), ProviderProtocol::OpenaiResponses) => {
            Err(InternalError::InvalidRoute(
                "openai_chat_completions -> openai_responses translation is not implemented yet"
                    .to_string(),
            ))
        }
        (
            PreparedInboundRequest::OpenaiChatCompletions(inbound),
            ProviderProtocol::AnthropicMessages,
        ) => {
            let translated = crate::translation::openai_chat_completions::to_anthropic_messages::translate_request_payload(
                &inbound.normalized_payload,
                &inbound.model,
                upstream_model,
            )?;
            Ok(ProviderRequest::anthropic_messages(
                anthropic_messages::request::prepare_provider_request(
                    &translated,
                    upstream_model,
                    upstream_model,
                )?,
                translated,
            ))
        }
        (PreparedInboundRequest::AnthropicMessages(inbound), ProviderProtocol::OpenaiResponses) => {
            let translated = crate::translation::anthropic_messages::to_openai_responses::translate_request_payload(
                &inbound.normalized_payload,
                &inbound.model,
                upstream_model,
            )?;
            Ok(ProviderRequest::openai_responses(
                openai_responses::request::prepare_provider_request(
                    &translated,
                    None,
                    upstream_model,
                    upstream_model,
                )?,
                translated,
            ))
        }
        (PreparedInboundRequest::AnthropicMessages(_), ProviderProtocol::OpenaiChatCompletions) => {
            Err(InternalError::InvalidRoute(
                "anthropic_messages -> openai_chat_completions translation is not implemented yet"
                    .to_string(),
            ))
        }
        (
            PreparedInboundRequest::AnthropicMessages(inbound),
            ProviderProtocol::AnthropicMessages,
        ) => Ok(ProviderRequest::anthropic_messages(
            anthropic_messages::request::prepare_provider_request(
                &inbound.normalized_payload,
                &inbound.model,
                upstream_model,
            )?,
            inbound.normalized_payload.clone(),
        )),
    }
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
