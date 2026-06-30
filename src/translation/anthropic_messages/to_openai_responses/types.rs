//! Basic type conversions for `anthropic_messages -> openai_responses`.
//!
//! Only protocol-to-protocol stateless conversions belong here. Pair-local ID
//! allocation lives in `ids.rs`, citation business logic lives in `citations.rs`,
//! and the pair-local ID string convention (`resp_…`) lives in `mod.rs`.

use crate::protocol::anthropic::messages::{
    RedactedThinkingBlock, ResponseServiceTier, StopReason, ThinkingBlock, ToolUseBlock, Usage,
};
use crate::protocol::openai_responses::{
    FunctionToolCall, InputTokenDetails, OutputStatus, OutputTokenDetails, ReasoningItem,
    ReasoningItemContent, ReasoningTextContent, ResponseUsage, ServiceTier, Status,
};
use crate::translation::TranslationResult;

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
