//! `openai_chat_completions -> anthropic_messages` request translation.

mod content;
mod request;
mod types;

use serde_json::Value;

use crate::protocol::anthropic::messages::MessageCreateParamsBase;
use crate::protocol::openai::chat_completions::CreateChatCompletionRequest;
use crate::translation::TranslationResult;

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let request = serde_json::from_value::<CreateChatCompletionRequest>(payload.clone())?;
    let translated: MessageCreateParamsBase = (&request).try_into()?;
    Ok(serde_json::to_value(translated)?)
}

#[cfg(test)]
mod tests;
