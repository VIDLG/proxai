use crate::protocol::anthropic::messages::{TextBlock, TextCitation};
use crate::protocol::openai::chat_completions::{
    ChatCompletionResponseMessageAnnotation, UrlCitation,
};

pub(super) fn text_block_annotations(
    block: &TextBlock,
    base_char_offset: usize,
) -> Vec<ChatCompletionResponseMessageAnnotation> {
    let mut search_start_byte = 0;
    block
        .citations
        .iter()
        .flatten()
        .filter_map(|citation| {
            citation_annotation(citation, block, base_char_offset, &mut search_start_byte)
        })
        .collect()
}

fn citation_annotation(
    citation: &TextCitation,
    block: &TextBlock,
    base_char_offset: usize,
    search_start_byte: &mut usize,
) -> Option<ChatCompletionResponseMessageAnnotation> {
    let TextCitation::WebSearchResultLocation(citation) = citation else {
        return None;
    };

    // Anthropic web citations are attached to the current text block and include
    // the cited text, but not an output-text offset. Search forward from the
    // previous matched citation so repeated cited text maps to later occurrences
    // in citation order instead of always selecting the first match.
    let matched_byte_offset = block.text[*search_start_byte..]
        .find(&citation.cited_text)
        .map(|relative_offset| *search_start_byte + relative_offset)?;
    *search_start_byte = matched_byte_offset.saturating_add(citation.cited_text.len());

    let text_offset = block.text[..matched_byte_offset].chars().count();
    let start = base_char_offset.saturating_add(text_offset);
    let end = start.saturating_add(citation.cited_text.chars().count());
    let start_index = u32::try_from(start).unwrap_or(u32::MAX);
    let end_index = u32::try_from(end).unwrap_or(u32::MAX);

    Some(ChatCompletionResponseMessageAnnotation::UrlCitation {
        url_citation: UrlCitation {
            start_index,
            end_index,
            title: citation.title.clone().unwrap_or_default(),
            url: citation.url.clone(),
        },
    })
}
