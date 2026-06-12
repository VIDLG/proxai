use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
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
            source: anthropic_document_source(file_data)?,
            cache_control: None,
            citations: None,
            context: None,
            title: file.filename.clone(),
        })
    }
}

fn anthropic_document_source(
    file_data: &str,
) -> TranslationResult<anthropic::DocumentBlockParamSource> {
    if let Some(data) = file_data.strip_prefix("data:application/pdf;base64,") {
        return Ok(anthropic::DocumentBlockParamSource::Base64(
            anthropic::Base64PdfSource {
                data: data.to_string(),
                media_type: anthropic::PdfMediaType::ApplicationPdf,
            },
        ));
    }

    if is_pdf_url(file_data) {
        return Ok(anthropic::DocumentBlockParamSource::Url(
            anthropic::UrlPdfSource {
                url: file_data.to_string(),
            },
        ));
    }

    Err(TranslationError::InvalidPayload(
        "Chat Completions file user content can only be translated to Anthropic Messages document content when `file_data` is a PDF data URL or PDF URL"
            .to_string(),
    ))
}

fn is_pdf_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let Some(path) = lower
        .strip_prefix("http://")
        .or_else(|| lower.strip_prefix("https://"))
    else {
        return false;
    };

    path.split_once('?')
        .map_or(path, |(path, _)| path)
        .ends_with(".pdf")
}
