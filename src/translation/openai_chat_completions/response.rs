use crate::protocol::openai::chat_completions::ChatChoice;
use crate::translation::{TranslationError, TranslationResult};

pub(crate) fn single_assistant_choice(choices: &[ChatChoice]) -> TranslationResult<&ChatChoice> {
    let choice = match choices {
        [] => {
            return Err(TranslationError::InvalidPayload(
                "Chat completion response has no choices to translate".to_string(),
            ));
        }
        [choice] => choice,
        choices => {
            return Err(TranslationError::InvalidPayload(format!(
                "Chat completion response has {} choices; target response can represent exactly one assistant message",
                choices.len()
            )));
        }
    };

    Ok(choice)
}
