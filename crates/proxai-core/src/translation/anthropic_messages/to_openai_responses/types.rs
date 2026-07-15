//! Basic type conversions for `anthropic_messages -> openai_responses`.
//!
//! Only protocol-to-protocol stateless conversions belong here. Pair-local ID
//! allocation lives in `ids.rs`, citation business logic lives in `citations.rs`,
//! and the pair-local ID string convention (`resp_…`) lives in `mod.rs`.

use crate::protocol::anthropic::messages::{
    RedactedThinkingBlock, ResponseServiceTier, StopReason, ThinkingBlock, ToolUseBlock, Usage,
};
use crate::protocol::openai_responses::{
    FunctionToolCall, IncompleteDetails, IncompleteDetailsReason, InputTokenDetails, OutputStatus,
    OutputTokenDetails, ReasoningItem, ReasoningItemContent, ReasoningTextContent, ResponseUsage,
    ServiceTier, Status,
};
use crate::translation::{TranslationError, TranslationResult};

/// Pair-local Responses `incomplete_details.reason` convention.
///
/// The string `"max_output_tokens"` is this pair's chosen wording (matching
/// OpenAI Responses API guidance), not an Anthropic protocol value.
pub(super) fn incomplete_details_from_stop_reason(
    stop_reason: Option<StopReason>,
) -> Option<IncompleteDetails> {
    match stop_reason {
        Some(StopReason::MaxTokens) => Some(IncompleteDetails {
            reason: Some(IncompleteDetailsReason::MaxOutputTokens),
        }),
        _ => None,
    }
}

impl From<&Usage> for ResponseUsage {
    fn from(usage: &Usage) -> Self {
        Self {
            input_tokens: usage.input_tokens,
            input_tokens_details: InputTokenDetails {
                cached_tokens: usage
                    .cache_read_input_tokens
                    .as_non_null()
                    .copied()
                    .unwrap_or_default(),
            },
            output_tokens: usage.output_tokens,
            output_tokens_details: OutputTokenDetails {
                reasoning_tokens: usage
                    .output_tokens_details
                    .as_non_null()
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

pub(super) fn responses_status_from_anthropic_stop_reason(stop_reason: StopReason) -> Status {
    match stop_reason {
        StopReason::MaxTokens => Status::Incomplete,
        StopReason::EndTurn
        | StopReason::StopSequence
        | StopReason::PauseTurn
        | StopReason::Refusal
        | StopReason::ToolUse => Status::Completed,
    }
}

impl TryFrom<&ToolUseBlock> for FunctionToolCall {
    type Error = TranslationError;

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

pub(super) fn reasoning_item_from_thinking(
    id: impl Into<String>,
    block: &ThinkingBlock,
) -> ReasoningItem {
    ReasoningItem {
        id: id.into(),
        summary: Vec::new(),
        content: Some(vec![ReasoningItemContent::ReasoningText(
            ReasoningTextContent {
                text: block.thinking.clone(),
            },
        )]),
        encrypted_content: None.into(),
        status: Some(OutputStatus::Completed),
    }
}

pub(super) fn reasoning_item_from_redacted_thinking(
    id: impl Into<String>,
    block: &RedactedThinkingBlock,
) -> ReasoningItem {
    ReasoningItem {
        id: id.into(),
        summary: Vec::new(),
        encrypted_content: Some(block.data.clone()).into(),
        content: None,
        status: Some(OutputStatus::Completed),
    }
}
