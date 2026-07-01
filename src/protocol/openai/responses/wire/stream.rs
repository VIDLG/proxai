use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::AsRefStr;

use super::{OutputContent, OutputItem, Response, ResponseLogProb, SummaryPart};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCreatedEvent {
    pub sequence_number: u64,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseInProgressEvent {
    pub sequence_number: u64,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCompletedEvent {
    pub sequence_number: u64,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFailedEvent {
    pub sequence_number: u64,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseIncompleteEvent {
    pub sequence_number: u64,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseOutputItemAddedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item: OutputItem,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseOutputItemDoneEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item: OutputItem,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseContentPartAddedEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub part: OutputContent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseContentPartDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub part: OutputContent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseTextDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub delta: String,
    pub logprobs: Option<Vec<ResponseLogProb>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseTextDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub text: String,
    pub logprobs: Option<Vec<ResponseLogProb>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseRefusalDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseRefusalDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub refusal: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFunctionCallArgumentsDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFunctionCallArgumentsDoneEvent {
    pub name: Option<String>,
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFileSearchCallInProgressEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFileSearchCallSearchingEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFileSearchCallCompletedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseWebSearchCallInProgressEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseWebSearchCallSearchingEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseWebSearchCallCompletedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseReasoningSummaryPartAddedEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub summary_index: u32,
    pub part: SummaryPart,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseReasoningSummaryPartDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub summary_index: u32,
    pub part: SummaryPart,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseReasoningSummaryTextDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub summary_index: u32,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseReasoningSummaryTextDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub summary_index: u32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseReasoningTextDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseReasoningTextDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseImageGenCallCompletedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseImageGenCallGeneratingEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseImageGenCallInProgressEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseImageGenCallPartialImageEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub partial_image_index: u32,
    pub partial_image_b64: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPCallArgumentsDeltaEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPCallArgumentsDoneEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPCallCompletedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPCallFailedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPCallInProgressEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPListToolsCompletedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPListToolsFailedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPListToolsInProgressEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCodeInterpreterCallInProgressEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCodeInterpreterCallInterpretingEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCodeInterpreterCallCompletedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCodeInterpreterCallCodeDeltaEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCodeInterpreterCallCodeDoneEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseOutputTextAnnotationAddedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub content_index: u32,
    pub annotation_index: u32,
    pub item_id: String,
    pub annotation: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseQueuedEvent {
    pub sequence_number: u64,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCustomToolCallInputDeltaEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCustomToolCallInputDoneEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub input: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseErrorEvent {
    pub sequence_number: u64,
    pub code: Option<String>,
    pub message: String,
    pub param: Option<String>,
}

#[allow(
    clippy::enum_variant_names,
    reason = "Mirrors OpenAI Responses stream event variant names."
)]
#[derive(Debug, Clone, PartialEq, AsRefStr, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseStreamEvent {
    #[serde(rename = "response.created")]
    #[strum(serialize = "response.created")]
    ResponseCreated(ResponseCreatedEvent),
    #[serde(rename = "response.in_progress")]
    #[strum(serialize = "response.in_progress")]
    ResponseInProgress(ResponseInProgressEvent),
    #[serde(rename = "response.completed")]
    #[strum(serialize = "response.completed")]
    ResponseCompleted(ResponseCompletedEvent),
    #[serde(rename = "response.failed")]
    #[strum(serialize = "response.failed")]
    ResponseFailed(ResponseFailedEvent),
    #[serde(rename = "response.incomplete")]
    #[strum(serialize = "response.incomplete")]
    ResponseIncomplete(ResponseIncompleteEvent),
    #[serde(rename = "response.output_item.added")]
    #[strum(serialize = "response.output_item.added")]
    ResponseOutputItemAdded(ResponseOutputItemAddedEvent),
    #[serde(rename = "response.output_item.done")]
    #[strum(serialize = "response.output_item.done")]
    ResponseOutputItemDone(ResponseOutputItemDoneEvent),
    #[serde(rename = "response.content_part.added")]
    #[strum(serialize = "response.content_part.added")]
    ResponseContentPartAdded(ResponseContentPartAddedEvent),
    #[serde(rename = "response.content_part.done")]
    #[strum(serialize = "response.content_part.done")]
    ResponseContentPartDone(ResponseContentPartDoneEvent),
    #[serde(rename = "response.output_text.delta")]
    #[strum(serialize = "response.output_text.delta")]
    ResponseOutputTextDelta(ResponseTextDeltaEvent),
    #[serde(rename = "response.output_text.done")]
    #[strum(serialize = "response.output_text.done")]
    ResponseOutputTextDone(ResponseTextDoneEvent),
    #[serde(rename = "response.refusal.delta")]
    #[strum(serialize = "response.refusal.delta")]
    ResponseRefusalDelta(ResponseRefusalDeltaEvent),
    #[serde(rename = "response.refusal.done")]
    #[strum(serialize = "response.refusal.done")]
    ResponseRefusalDone(ResponseRefusalDoneEvent),
    #[serde(rename = "response.function_call_arguments.delta")]
    #[strum(serialize = "response.function_call_arguments.delta")]
    ResponseFunctionCallArgumentsDelta(ResponseFunctionCallArgumentsDeltaEvent),
    #[serde(rename = "response.function_call_arguments.done")]
    #[strum(serialize = "response.function_call_arguments.done")]
    ResponseFunctionCallArgumentsDone(ResponseFunctionCallArgumentsDoneEvent),
    #[serde(rename = "response.file_search_call.in_progress")]
    #[strum(serialize = "response.file_search_call.in_progress")]
    ResponseFileSearchCallInProgress(ResponseFileSearchCallInProgressEvent),
    #[serde(rename = "response.file_search_call.searching")]
    #[strum(serialize = "response.file_search_call.searching")]
    ResponseFileSearchCallSearching(ResponseFileSearchCallSearchingEvent),
    #[serde(rename = "response.file_search_call.completed")]
    #[strum(serialize = "response.file_search_call.completed")]
    ResponseFileSearchCallCompleted(ResponseFileSearchCallCompletedEvent),
    #[serde(rename = "response.web_search_call.in_progress")]
    #[strum(serialize = "response.web_search_call.in_progress")]
    ResponseWebSearchCallInProgress(ResponseWebSearchCallInProgressEvent),
    #[serde(rename = "response.web_search_call.searching")]
    #[strum(serialize = "response.web_search_call.searching")]
    ResponseWebSearchCallSearching(ResponseWebSearchCallSearchingEvent),
    #[serde(rename = "response.web_search_call.completed")]
    #[strum(serialize = "response.web_search_call.completed")]
    ResponseWebSearchCallCompleted(ResponseWebSearchCallCompletedEvent),
    #[serde(rename = "response.reasoning_summary_part.added")]
    #[strum(serialize = "response.reasoning_summary_part.added")]
    ResponseReasoningSummaryPartAdded(ResponseReasoningSummaryPartAddedEvent),
    #[serde(rename = "response.reasoning_summary_part.done")]
    #[strum(serialize = "response.reasoning_summary_part.done")]
    ResponseReasoningSummaryPartDone(ResponseReasoningSummaryPartDoneEvent),
    #[serde(rename = "response.reasoning_summary_text.delta")]
    #[strum(serialize = "response.reasoning_summary_text.delta")]
    ResponseReasoningSummaryTextDelta(ResponseReasoningSummaryTextDeltaEvent),
    #[serde(rename = "response.reasoning_summary_text.done")]
    #[strum(serialize = "response.reasoning_summary_text.done")]
    ResponseReasoningSummaryTextDone(ResponseReasoningSummaryTextDoneEvent),
    #[serde(rename = "response.reasoning_text.delta")]
    #[strum(serialize = "response.reasoning_text.delta")]
    ResponseReasoningTextDelta(ResponseReasoningTextDeltaEvent),
    #[serde(rename = "response.reasoning_text.done")]
    #[strum(serialize = "response.reasoning_text.done")]
    ResponseReasoningTextDone(ResponseReasoningTextDoneEvent),
    #[serde(rename = "response.image_generation_call.completed")]
    #[strum(serialize = "response.image_generation_call.completed")]
    ResponseImageGenerationCallCompleted(ResponseImageGenCallCompletedEvent),
    #[serde(rename = "response.image_generation_call.generating")]
    #[strum(serialize = "response.image_generation_call.generating")]
    ResponseImageGenerationCallGenerating(ResponseImageGenCallGeneratingEvent),
    #[serde(rename = "response.image_generation_call.in_progress")]
    #[strum(serialize = "response.image_generation_call.in_progress")]
    ResponseImageGenerationCallInProgress(ResponseImageGenCallInProgressEvent),
    #[serde(rename = "response.image_generation_call.partial_image")]
    #[strum(serialize = "response.image_generation_call.partial_image")]
    ResponseImageGenerationCallPartialImage(ResponseImageGenCallPartialImageEvent),
    #[serde(rename = "response.mcp_call.arguments_delta")]
    #[strum(serialize = "response.mcp_call.arguments_delta")]
    ResponseMCPCallArgumentsDelta(ResponseMCPCallArgumentsDeltaEvent),
    #[serde(rename = "response.mcp_call.arguments_done")]
    #[strum(serialize = "response.mcp_call.arguments_done")]
    ResponseMCPCallArgumentsDone(ResponseMCPCallArgumentsDoneEvent),
    #[serde(rename = "response.mcp_call.completed")]
    #[strum(serialize = "response.mcp_call.completed")]
    ResponseMCPCallCompleted(ResponseMCPCallCompletedEvent),
    #[serde(rename = "response.mcp_call.failed")]
    #[strum(serialize = "response.mcp_call.failed")]
    ResponseMCPCallFailed(ResponseMCPCallFailedEvent),
    #[serde(rename = "response.mcp_call.in_progress")]
    #[strum(serialize = "response.mcp_call.in_progress")]
    ResponseMCPCallInProgress(ResponseMCPCallInProgressEvent),
    #[serde(rename = "response.mcp_list_tools.completed")]
    #[strum(serialize = "response.mcp_list_tools.completed")]
    ResponseMCPListToolsCompleted(ResponseMCPListToolsCompletedEvent),
    #[serde(rename = "response.mcp_list_tools.failed")]
    #[strum(serialize = "response.mcp_list_tools.failed")]
    ResponseMCPListToolsFailed(ResponseMCPListToolsFailedEvent),
    #[serde(rename = "response.mcp_list_tools.in_progress")]
    #[strum(serialize = "response.mcp_list_tools.in_progress")]
    ResponseMCPListToolsInProgress(ResponseMCPListToolsInProgressEvent),
    #[serde(rename = "response.code_interpreter_call.in_progress")]
    #[strum(serialize = "response.code_interpreter_call.in_progress")]
    ResponseCodeInterpreterCallInProgress(ResponseCodeInterpreterCallInProgressEvent),
    #[serde(rename = "response.code_interpreter_call.interpreting")]
    #[strum(serialize = "response.code_interpreter_call.interpreting")]
    ResponseCodeInterpreterCallInterpreting(ResponseCodeInterpreterCallInterpretingEvent),
    #[serde(rename = "response.code_interpreter_call.completed")]
    #[strum(serialize = "response.code_interpreter_call.completed")]
    ResponseCodeInterpreterCallCompleted(ResponseCodeInterpreterCallCompletedEvent),
    #[serde(rename = "response.code_interpreter_call.code_delta")]
    #[strum(serialize = "response.code_interpreter_call.code_delta")]
    ResponseCodeInterpreterCallCodeDelta(ResponseCodeInterpreterCallCodeDeltaEvent),
    #[serde(rename = "response.code_interpreter_call.code_done")]
    #[strum(serialize = "response.code_interpreter_call.code_done")]
    ResponseCodeInterpreterCallCodeDone(ResponseCodeInterpreterCallCodeDoneEvent),
    #[serde(rename = "response.output_text_annotation.added")]
    #[strum(serialize = "response.output_text_annotation.added")]
    ResponseOutputTextAnnotationAdded(ResponseOutputTextAnnotationAddedEvent),
    #[serde(rename = "response.queued")]
    #[strum(serialize = "response.queued")]
    ResponseQueued(ResponseQueuedEvent),
    #[serde(rename = "response.custom_tool_call_input.delta")]
    #[strum(serialize = "response.custom_tool_call_input.delta")]
    ResponseCustomToolCallInputDelta(ResponseCustomToolCallInputDeltaEvent),
    #[serde(rename = "response.custom_tool_call_input.done")]
    #[strum(serialize = "response.custom_tool_call_input.done")]
    ResponseCustomToolCallInputDone(ResponseCustomToolCallInputDoneEvent),
    #[serde(rename = "response.error")]
    #[strum(serialize = "response.error")]
    ResponseError(ResponseErrorEvent),
}

impl ResponseStreamEvent {
    pub fn sequence_number(&self) -> u64 {
        match self {
            Self::ResponseCreated(event) => event.sequence_number,
            Self::ResponseInProgress(event) => event.sequence_number,
            Self::ResponseCompleted(event) => event.sequence_number,
            Self::ResponseFailed(event) => event.sequence_number,
            Self::ResponseIncomplete(event) => event.sequence_number,
            Self::ResponseOutputItemAdded(event) => event.sequence_number,
            Self::ResponseOutputItemDone(event) => event.sequence_number,
            Self::ResponseContentPartAdded(event) => event.sequence_number,
            Self::ResponseContentPartDone(event) => event.sequence_number,
            Self::ResponseOutputTextDelta(event) => event.sequence_number,
            Self::ResponseOutputTextDone(event) => event.sequence_number,
            Self::ResponseRefusalDelta(event) => event.sequence_number,
            Self::ResponseRefusalDone(event) => event.sequence_number,
            Self::ResponseFunctionCallArgumentsDelta(event) => event.sequence_number,
            Self::ResponseFunctionCallArgumentsDone(event) => event.sequence_number,
            Self::ResponseFileSearchCallInProgress(event) => event.sequence_number,
            Self::ResponseFileSearchCallSearching(event) => event.sequence_number,
            Self::ResponseFileSearchCallCompleted(event) => event.sequence_number,
            Self::ResponseWebSearchCallInProgress(event) => event.sequence_number,
            Self::ResponseWebSearchCallSearching(event) => event.sequence_number,
            Self::ResponseWebSearchCallCompleted(event) => event.sequence_number,
            Self::ResponseReasoningSummaryPartAdded(event) => event.sequence_number,
            Self::ResponseReasoningSummaryPartDone(event) => event.sequence_number,
            Self::ResponseReasoningSummaryTextDelta(event) => event.sequence_number,
            Self::ResponseReasoningSummaryTextDone(event) => event.sequence_number,
            Self::ResponseReasoningTextDelta(event) => event.sequence_number,
            Self::ResponseReasoningTextDone(event) => event.sequence_number,
            Self::ResponseImageGenerationCallCompleted(event) => event.sequence_number,
            Self::ResponseImageGenerationCallGenerating(event) => event.sequence_number,
            Self::ResponseImageGenerationCallInProgress(event) => event.sequence_number,
            Self::ResponseImageGenerationCallPartialImage(event) => event.sequence_number,
            Self::ResponseMCPCallArgumentsDelta(event) => event.sequence_number,
            Self::ResponseMCPCallArgumentsDone(event) => event.sequence_number,
            Self::ResponseMCPCallCompleted(event) => event.sequence_number,
            Self::ResponseMCPCallFailed(event) => event.sequence_number,
            Self::ResponseMCPCallInProgress(event) => event.sequence_number,
            Self::ResponseMCPListToolsCompleted(event) => event.sequence_number,
            Self::ResponseMCPListToolsFailed(event) => event.sequence_number,
            Self::ResponseMCPListToolsInProgress(event) => event.sequence_number,
            Self::ResponseCodeInterpreterCallInProgress(event) => event.sequence_number,
            Self::ResponseCodeInterpreterCallInterpreting(event) => event.sequence_number,
            Self::ResponseCodeInterpreterCallCompleted(event) => event.sequence_number,
            Self::ResponseCodeInterpreterCallCodeDelta(event) => event.sequence_number,
            Self::ResponseCodeInterpreterCallCodeDone(event) => event.sequence_number,
            Self::ResponseOutputTextAnnotationAdded(event) => event.sequence_number,
            Self::ResponseQueued(event) => event.sequence_number,
            Self::ResponseCustomToolCallInputDelta(event) => event.sequence_number,
            Self::ResponseCustomToolCallInputDone(event) => event.sequence_number,
            Self::ResponseError(event) => event.sequence_number,
        }
    }
}

// ── Stream request options ────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ResponseStreamOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_obfuscation: Option<bool>,
}
