use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::translation::anthropic_messages::outbound::pdf_document_source_from_file_data_or_url;
use crate::translation::{TranslationError, TranslationResult};

impl TryFrom<&chat::FileObject> for anthropic::DocumentBlockParam {
    type Error = TranslationError;

    fn try_from(file: &chat::FileObject) -> TranslationResult<Self> {
        let Some(file_data) = file.file_data.as_deref() else {
            return Err(TranslationError::InvalidPayload(
                "Chat Completions file user content with only file_id cannot be translated to Anthropic Messages document content"
                    .to_string(),
            ));
        };

        Ok(anthropic::DocumentBlockParam {
            source: pdf_document_source_from_file_data_or_url(file_data)?,
            cache_control: None.into(),
            citations: None.into(),
            context: None.into(),
            title: file.filename.clone().into(),
        })
    }
}
