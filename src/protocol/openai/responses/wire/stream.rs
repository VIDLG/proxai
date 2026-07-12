use crate::protocol::RequiredNullable;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::AsRefStr;

use super::{OutputContent, OutputItem, Response, ResponseLogProb, SummaryPart};

/// OpenAPI schema: `#/components/schemas/ResponseCreatedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCreatedEvent {
    pub response: Response,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseInProgressEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseInProgressEvent {
    pub response: Response,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseCompletedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCompletedEvent {
    pub response: Response,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseFailedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFailedEvent {
    pub sequence_number: u64,
    pub response: Response,
}

/// OpenAPI schema: `#/components/schemas/ResponseIncompleteEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseIncompleteEvent {
    pub response: Response,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseOutputItemAddedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseOutputItemAddedEvent {
    pub output_index: u32,

    pub sequence_number: u64,
    pub item: OutputItem,
}

/// OpenAPI schema: `#/components/schemas/ResponseOutputItemDoneEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseOutputItemDoneEvent {
    pub output_index: u32,

    pub sequence_number: u64,
    pub item: OutputItem,
}

/// OpenAPI schema: `#/components/schemas/ResponseContentPartAddedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseContentPartAddedEvent {
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub part: OutputContent,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseContentPartDoneEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseContentPartDoneEvent {
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,

    pub sequence_number: u64,
    pub part: OutputContent,
}

/// OpenAPI schema: `#/components/schemas/ResponseTextDeltaEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseTextDeltaEvent {
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub delta: String,

    pub sequence_number: u64,
    pub logprobs: Vec<ResponseLogProb>,
}

/// OpenAPI schema: `#/components/schemas/ResponseTextDoneEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseTextDoneEvent {
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub text: String,

    pub sequence_number: u64,
    pub logprobs: Vec<ResponseLogProb>,
}

/// OpenAPI schema: `#/components/schemas/ResponseRefusalDeltaEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseRefusalDeltaEvent {
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub delta: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseRefusalDoneEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseRefusalDoneEvent {
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub refusal: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseFunctionCallArgumentsDeltaEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFunctionCallArgumentsDeltaEvent {
    pub item_id: String,
    pub output_index: u32,

    pub sequence_number: u64,
    pub delta: String,
}

/// OpenAPI schema: `#/components/schemas/ResponseFunctionCallArgumentsDoneEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFunctionCallArgumentsDoneEvent {
    pub item_id: String,

    pub name: String,
    pub output_index: u32,
    pub sequence_number: u64,
    pub arguments: String,
}

/// OpenAPI schema: `#/components/schemas/ResponseFileSearchCallInProgressEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFileSearchCallInProgressEvent {
    pub output_index: u32,
    pub item_id: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseFileSearchCallSearchingEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFileSearchCallSearchingEvent {
    pub output_index: u32,
    pub item_id: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseFileSearchCallCompletedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFileSearchCallCompletedEvent {
    pub output_index: u32,
    pub item_id: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseWebSearchCallInProgressEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseWebSearchCallInProgressEvent {
    pub output_index: u32,
    pub item_id: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseWebSearchCallSearchingEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseWebSearchCallSearchingEvent {
    pub output_index: u32,
    pub item_id: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseWebSearchCallCompletedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseWebSearchCallCompletedEvent {
    pub output_index: u32,
    pub item_id: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseReasoningSummaryPartAddedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseReasoningSummaryPartAddedEvent {
    pub item_id: String,
    pub output_index: u32,
    pub summary_index: u32,

    pub sequence_number: u64,
    pub part: SummaryPart,
}

/// OpenAPI schema: `#/components/schemas/ResponseReasoningSummaryPartDoneEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseReasoningSummaryPartDoneEvent {
    pub item_id: String,
    pub output_index: u32,
    pub summary_index: u32,

    pub sequence_number: u64,
    pub part: SummaryPart,
}

/// OpenAPI schema: `#/components/schemas/ResponseReasoningSummaryTextDeltaEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseReasoningSummaryTextDeltaEvent {
    pub item_id: String,
    pub output_index: u32,
    pub summary_index: u32,
    pub delta: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseReasoningSummaryTextDoneEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseReasoningSummaryTextDoneEvent {
    pub item_id: String,
    pub output_index: u32,
    pub summary_index: u32,
    pub text: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseReasoningTextDeltaEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseReasoningTextDeltaEvent {
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub delta: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseReasoningTextDoneEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseReasoningTextDoneEvent {
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub text: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseImageGenCallCompletedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseImageGenCallCompletedEvent {
    pub output_index: u32,

    pub sequence_number: u64,
    pub item_id: String,
}

/// OpenAPI schema: `#/components/schemas/ResponseImageGenCallGeneratingEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseImageGenCallGeneratingEvent {
    pub output_index: u32,
    pub item_id: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseImageGenCallInProgressEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseImageGenCallInProgressEvent {
    pub output_index: u32,
    pub item_id: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseImageGenCallPartialImageEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseImageGenCallPartialImageEvent {
    pub output_index: u32,
    pub item_id: String,

    pub sequence_number: u64,
    pub partial_image_index: u32,
    pub partial_image_b64: String,
}

/// OpenAPI schema: `#/components/schemas/ResponseMCPCallArgumentsDeltaEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPCallArgumentsDeltaEvent {
    pub output_index: u32,
    pub item_id: String,
    pub delta: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseMCPCallArgumentsDoneEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPCallArgumentsDoneEvent {
    pub output_index: u32,
    pub item_id: String,
    pub arguments: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseMCPCallCompletedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPCallCompletedEvent {
    pub item_id: String,
    pub output_index: u32,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseMCPCallFailedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPCallFailedEvent {
    pub item_id: String,
    pub output_index: u32,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseMCPCallInProgressEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPCallInProgressEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

/// OpenAPI schema: `#/components/schemas/ResponseMCPListToolsCompletedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPListToolsCompletedEvent {
    pub item_id: String,
    pub output_index: u32,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseMCPListToolsFailedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPListToolsFailedEvent {
    pub item_id: String,
    pub output_index: u32,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseMCPListToolsInProgressEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMCPListToolsInProgressEvent {
    pub item_id: String,
    pub output_index: u32,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseCodeInterpreterCallInProgressEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCodeInterpreterCallInProgressEvent {
    pub output_index: u32,
    pub item_id: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseCodeInterpreterCallInterpretingEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCodeInterpreterCallInterpretingEvent {
    pub output_index: u32,
    pub item_id: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseCodeInterpreterCallCompletedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCodeInterpreterCallCompletedEvent {
    pub output_index: u32,
    pub item_id: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseCodeInterpreterCallCodeDeltaEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCodeInterpreterCallCodeDeltaEvent {
    pub output_index: u32,
    pub item_id: String,
    pub delta: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseCodeInterpreterCallCodeDoneEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCodeInterpreterCallCodeDoneEvent {
    pub output_index: u32,
    pub item_id: String,
    pub code: String,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseOutputTextAnnotationAddedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseOutputTextAnnotationAddedEvent {
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub annotation_index: u32,

    pub sequence_number: u64,
    pub annotation: Value,
}

/// OpenAPI schema: `#/components/schemas/ResponseQueuedEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseQueuedEvent {
    pub response: Response,

    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseCustomToolCallInputDeltaEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCustomToolCallInputDeltaEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub delta: String,
}

/// OpenAPI schema: `#/components/schemas/ResponseCustomToolCallInputDoneEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCustomToolCallInputDoneEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub input: String,
}

/// OpenAPI schema: `#/components/schemas/ResponseAudioDeltaEvent`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseAudioDeltaEvent {
    pub sequence_number: u64,
    pub delta: String,
}

/// OpenAPI schema: `#/components/schemas/ResponseAudioDoneEvent`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseAudioDoneEvent {
    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseAudioTranscriptDeltaEvent`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseAudioTranscriptDeltaEvent {
    pub delta: String,
    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseAudioTranscriptDoneEvent`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseAudioTranscriptDoneEvent {
    pub sequence_number: u64,
}

/// OpenAPI schema: `#/components/schemas/ResponseErrorEvent`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseErrorEvent {
    pub code: RequiredNullable<String>,
    pub message: String,
    pub param: RequiredNullable<String>,

    pub sequence_number: u64,
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
    #[serde(rename = "response.mcp_call_arguments.delta")]
    #[strum(serialize = "response.mcp_call_arguments.delta")]
    ResponseMCPCallArgumentsDelta(ResponseMCPCallArgumentsDeltaEvent),
    #[serde(rename = "response.mcp_call_arguments.done")]
    #[strum(serialize = "response.mcp_call_arguments.done")]
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
    #[serde(rename = "response.code_interpreter_call_code.delta")]
    #[strum(serialize = "response.code_interpreter_call_code.delta")]
    ResponseCodeInterpreterCallCodeDelta(ResponseCodeInterpreterCallCodeDeltaEvent),
    #[serde(rename = "response.code_interpreter_call_code.done")]
    #[strum(serialize = "response.code_interpreter_call_code.done")]
    ResponseCodeInterpreterCallCodeDone(ResponseCodeInterpreterCallCodeDoneEvent),
    #[serde(rename = "response.output_text.annotation.added")]
    #[strum(serialize = "response.output_text.annotation.added")]
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
    #[serde(rename = "response.audio.delta")]
    #[strum(serialize = "response.audio.delta")]
    ResponseAudioDelta(ResponseAudioDeltaEvent),
    #[serde(rename = "response.audio.done")]
    #[strum(serialize = "response.audio.done")]
    ResponseAudioDone(ResponseAudioDoneEvent),
    #[serde(rename = "response.audio.transcript.delta")]
    #[strum(serialize = "response.audio.transcript.delta")]
    ResponseAudioTranscriptDelta(ResponseAudioTranscriptDeltaEvent),
    #[serde(rename = "response.audio.transcript.done")]
    #[strum(serialize = "response.audio.transcript.done")]
    ResponseAudioTranscriptDone(ResponseAudioTranscriptDoneEvent),
    #[serde(rename = "error")]
    #[strum(serialize = "error")]
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
            Self::ResponseAudioDelta(event) => event.sequence_number,
            Self::ResponseAudioDone(event) => event.sequence_number,
            Self::ResponseAudioTranscriptDelta(event) => event.sequence_number,
            Self::ResponseAudioTranscriptDone(event) => event.sequence_number,
            Self::ResponseError(event) => event.sequence_number,
        }
    }
}

// ── Stream request options ────────────────────────────────────

/// OpenAPI schema: `#/components/schemas/ResponseStreamOptions`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ResponseStreamOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_obfuscation: Option<bool>,
}
