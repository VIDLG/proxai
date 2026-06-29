//! Basic type conversions for `anthropic_messages -> openai_responses`.

use crate::protocol::anthropic::messages::{
    RedactedThinkingBlock, ResponseServiceTier, StopReason, TextBlock, TextCitation, ThinkingBlock,
    ToolUseBlock, Usage,
};
use crate::protocol::openai_responses::{
    Annotation, FunctionToolCall, IncompleteDetails, InputTokenDetails, OutputStatus,
    OutputTokenDetails, ReasoningItem, ReasoningItemContent, ReasoningTextContent, ResponseUsage,
    ServiceTier, Status, UrlCitationBody,
};
use crate::translation::TranslationResult;

pub(super) fn response_id(message_id: &str) -> String {
    if message_id.starts_with("resp_") {
        message_id.to_string()
    } else {
        format!("resp_{message_id}")
    }
}

impl From<&Usage> for ResponseUsage {
    fn from(usage: &Usage) -> Self {
        Self {
            input_tokens: usage.input_tokens,
            input_tokens_details: InputTokenDetails {
                cached_tokens: usage.cache_read_input_tokens.unwrap_or_default(),
            },
            output_tokens: usage.output_tokens,
            output_tokens_details: OutputTokenDetails {
                reasoning_tokens: usage
                    .output_tokens_details
                    .as_ref()
                    .map_or(0, |d| d.thinking_tokens),
            },
            total_tokens: usage.input_tokens.saturating_add(usage.output_tokens),
        }
    }
}

impl From<ResponseServiceTier> for Option<ServiceTier> {
    fn from(service_tier: ResponseServiceTier) -> Self {
        match service_tier {
            ResponseServiceTier::Standard => Some(ServiceTier::Default),
            ResponseServiceTier::Priority => Some(ServiceTier::Priority),
            ResponseServiceTier::Batch => None,
        }
    }
}

impl From<StopReason> for Status {
    fn from(stop_reason: StopReason) -> Self {
        match stop_reason {
            StopReason::MaxTokens => Status::Incomplete,
            StopReason::Refusal => Status::Failed,
            StopReason::EndTurn
            | StopReason::StopSequence
            | StopReason::PauseTurn
            | StopReason::ToolUse => Status::Completed,
        }
    }
}

pub(super) fn incomplete_details_from_stop_reason(
    stop_reason: Option<StopReason>,
) -> Option<IncompleteDetails> {
    match stop_reason {
        Some(StopReason::MaxTokens) => Some(IncompleteDetails {
            reason: "max_output_tokens".to_string(),
        }),
        _ => None,
    }
}

#[derive(Debug)]
pub(super) struct OutputItemIdAllocator {
    message_id: String,
    next_message_index: u32,
    next_reasoning_index: u32,
    next_function_call_output_index: u32,
}

impl OutputItemIdAllocator {
    pub(super) fn new(message_id: impl Into<String>) -> Self {
        Self {
            message_id: message_id.into(),
            next_message_index: 0,
            next_reasoning_index: 0,
            next_function_call_output_index: 0,
        }
    }

    pub(super) fn message(&mut self) -> String {
        let id = Self::indexed_id("msg", &self.message_id, self.next_message_index);
        self.next_message_index = self.next_message_index.saturating_add(1);
        id
    }

    pub(super) fn reasoning(&mut self) -> String {
        let id = Self::indexed_id("rs", &self.message_id, self.next_reasoning_index);
        self.next_reasoning_index = self.next_reasoning_index.saturating_add(1);
        id
    }

    pub(super) fn function_call_output(&mut self) -> String {
        let id = Self::indexed_id(
            "fco",
            &self.message_id,
            self.next_function_call_output_index,
        );
        self.next_function_call_output_index =
            self.next_function_call_output_index.saturating_add(1);
        id
    }

    fn indexed_id(prefix: &str, message_id: &str, index: u32) -> String {
        if index == 0 {
            format!("{prefix}_{message_id}")
        } else {
            format!("{prefix}_{message_id}_{index}")
        }
    }
}

impl TryFrom<&ToolUseBlock> for FunctionToolCall {
    type Error = crate::translation::TranslationError;

    fn try_from(block: &ToolUseBlock) -> TranslationResult<Self> {
        Ok(Self {
            id: Some(block.id.clone()),
            call_id: block.id.clone(),
            name: block.name.clone(),
            arguments: serde_json::to_string(&block.input)?,
            status: Some(OutputStatus::Completed),
            namespace: None,
        })
    }
}

impl From<&ThinkingBlock> for ReasoningItem {
    fn from(block: &ThinkingBlock) -> Self {
        Self {
            id: None,
            summary: Vec::new(),
            content: Some(vec![ReasoningItemContent::ReasoningText(
                ReasoningTextContent {
                    text: block.thinking.clone(),
                },
            )]),
            encrypted_content: None,
            status: Some(OutputStatus::Completed),
        }
    }
}

impl From<&RedactedThinkingBlock> for ReasoningItem {
    fn from(block: &RedactedThinkingBlock) -> Self {
        Self {
            id: None,
            summary: Vec::new(),
            encrypted_content: Some(block.data.clone()),
            content: None,
            status: Some(OutputStatus::Completed),
        }
    }
}

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

/// Convert an Anthropic `TextCitation` into an OpenAI Responses `Annotation`.
///
/// Only `WebSearchResultLocation` citations carry a URL and can be losslessly
/// mapped to `UrlCitation`. Other citation types are skipped with a trace log.
///
/// The function maps the cited text span to character offsets within the
/// combined output text across multiple `TextBlock`s:
///
/// - `search_start_byte` is a advancing cursor that prevents repeated matches
///   when the same cited text appears multiple times in the block.
/// - `base_char_offset` accumulates the character count from previous blocks
///   so the resulting indices are relative to the full output, not just the
///   current block.
/// - UTF-8 byte offsets from `str::find()` are converted to character offsets
///   via `.chars().count()` because the Responses API uses character indices.
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
