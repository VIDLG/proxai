use serde_json::Value;

use crate::protocol::ProviderProtocol;

/// Provider-reported error details normalized from a structured JSON payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderError {
    pub code: Option<String>,
    pub message: String,
    pub param: Option<Value>,
}

/// Normalize a structured provider error payload without depending on HTTP
/// status, headers, body bytes, or client-facing rendering.
///
/// Protocol-specific official shapes are preferred, followed by conservative
/// compatibility fallbacks used by OpenAI-compatible and FastAPI-style
/// upstreams.
pub fn normalize_provider_error(
    protocol: ProviderProtocol,
    payload: &Value,
) -> Option<ProviderError> {
    let code = match protocol {
        ProviderProtocol::AnthropicMessages => payload
            .pointer("/error/type")
            .or_else(|| payload.pointer("/error/code")),
        ProviderProtocol::OpenaiResponses | ProviderProtocol::OpenaiChatCompletions => {
            payload.pointer("/error/code")
        }
    }
    .or_else(|| payload.pointer("/code"))
    .and_then(Value::as_str)
    .map(str::to_string);
    let param = payload
        .pointer("/error/param")
        .or_else(|| payload.pointer("/param"))
        .cloned();

    if let Some(message) = payload
        .pointer("/error/message")
        .or_else(|| payload.pointer("/error"))
        .or_else(|| payload.pointer("/detail"))
        .or_else(|| payload.pointer("/message"))
        .and_then(Value::as_str)
    {
        return Some(ProviderError {
            code,
            message: message.to_string(),
            param,
        });
    }

    let message = payload
        .pointer("/detail")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            item.get("msg")
                .or_else(|| item.get("message"))
                .and_then(Value::as_str)
        })
        .collect::<Vec<_>>()
        .join("; ");
    (!message.is_empty()).then_some(ProviderError {
        code,
        message,
        param,
    })
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
