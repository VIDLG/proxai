use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::chat_completions::request::wire::ResponseFormatJsonSchema;

pub(super) fn chat_stop_configuration(
    stop_sequences: Option<Vec<String>>,
) -> Option<chat::StopConfiguration> {
    let mut stop_sequences = stop_sequences?
        .into_iter()
        .filter(|sequence| !sequence.is_empty())
        .collect::<Vec<_>>();
    match stop_sequences.len() {
        0 => None,
        1 => stop_sequences.pop().map(chat::StopConfiguration::String),
        _ => Some(chat::StopConfiguration::StringArray(stop_sequences)),
    }
}

pub(super) fn non_empty<T>(items: Vec<T>) -> Option<Vec<T>> {
    (!items.is_empty()).then_some(items)
}

impl From<anthropic::OutputFormat> for chat::ResponseFormat {
    fn from(format: anthropic::OutputFormat) -> Self {
        match format {
            anthropic::OutputFormat::JsonSchema(schema) => Self::JsonSchema {
                json_schema: ResponseFormatJsonSchema {
                    description: None,
                    name: "anthropic_json_schema".to_string(),
                    schema: Some(schema.schema),
                    strict: None,
                },
            },
        }
    }
}

impl From<anthropic::RequestServiceTier> for chat::ServiceTier {
    fn from(tier: anthropic::RequestServiceTier) -> Self {
        match tier {
            anthropic::RequestServiceTier::Auto => Self::Auto,
            anthropic::RequestServiceTier::StandardOnly => Self::Default,
        }
    }
}
