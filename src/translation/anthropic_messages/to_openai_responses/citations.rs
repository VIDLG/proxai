//! Anthropic web citations -> OpenAI Responses URL annotations.
//!
//! Citations carry the cited text but no output-text offset, so each citation
//! is mapped by searching the cited text inside its owning block (advancing
//! past previous matches so repeated cited text maps to later occurrences) and
//! then accumulated against a per-message character cursor so the resulting
//! indices are relative to the full output text, not just the current block.

use crate::protocol::anthropic::messages::{TextBlock, TextCitation};
use crate::protocol::openai_responses::{Annotation, UrlCitationBody};

pub(super) fn text_block_annotations(
    block: &TextBlock,
    base_char_offset: usize,
) -> Vec<Annotation> {
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
) -> Option<Annotation> {
    let TextCitation::WebSearchResultLocation(citation) = citation else {
        let discriminant = std::mem::discriminant(citation);
        tracing::trace!(?discriminant, "unsupported citation type, skipping");
        return None;
    };

    // Find the cited text starting from the last matched position so repeated
    // occurrences map to later positions in citation order.
    let matched_byte_offset = block.text[*search_start_byte..]
        .find(&citation.cited_text)
        .map(|relative_offset| *search_start_byte + relative_offset)?;

    // Advance the cursor past this match to handle duplicate cited text.
    *search_start_byte = matched_byte_offset.saturating_add(citation.cited_text.len());

    // Convert byte offset to character offset (UTF-8 multi-byte safe).
    let text_offset = block.text[..matched_byte_offset].chars().count();

    // Absolute character position in the full output = block prefix + local offset.
    let start = base_char_offset.saturating_add(text_offset);
    let end = start.saturating_add(citation.cited_text.chars().count());
    let start_index = u32::try_from(start).unwrap_or(u32::MAX);
    let end_index = u32::try_from(end).unwrap_or(u32::MAX);

    Some(Annotation::UrlCitation(UrlCitationBody {
        start_index,
        end_index,
        title: citation.title.clone().unwrap_or_default(),
        url: citation.url.clone(),
    }))
}
