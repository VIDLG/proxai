use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use structural_convert::StructuralConvert;

use super::{OutputContent, OutputItem, Response, ResponseLogProb, SummaryPart};

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseCreatedEvent))]
pub struct ResponseCreatedEvent {
    pub sequence_number: u64,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseInProgressEvent))]
pub struct ResponseInProgressEvent {
    pub sequence_number: u64,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseCompletedEvent))]
pub struct ResponseCompletedEvent {
    pub sequence_number: u64,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseFailedEvent))]
pub struct ResponseFailedEvent {
    pub sequence_number: u64,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseIncompleteEvent))]
pub struct ResponseIncompleteEvent {
    pub sequence_number: u64,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseOutputItemAddedEvent))]
pub struct ResponseOutputItemAddedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item: OutputItem,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseOutputItemDoneEvent))]
pub struct ResponseOutputItemDoneEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item: OutputItem,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseContentPartAddedEvent))]
pub struct ResponseContentPartAddedEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub part: OutputContent,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseContentPartDoneEvent))]
pub struct ResponseContentPartDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub part: OutputContent,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseTextDeltaEvent))]
pub struct ResponseTextDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub delta: String,
    pub logprobs: Option<Vec<ResponseLogProb>>,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseTextDoneEvent))]
pub struct ResponseTextDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub text: String,
    pub logprobs: Option<Vec<ResponseLogProb>>,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseRefusalDeltaEvent))]
pub struct ResponseRefusalDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseRefusalDoneEvent))]
pub struct ResponseRefusalDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub refusal: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseFunctionCallArgumentsDeltaEvent))]
pub struct ResponseFunctionCallArgumentsDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseFunctionCallArgumentsDoneEvent))]
pub struct ResponseFunctionCallArgumentsDoneEvent {
    pub name: Option<String>,
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseFileSearchCallInProgressEvent))]
pub struct ResponseFileSearchCallInProgressEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseFileSearchCallSearchingEvent))]
pub struct ResponseFileSearchCallSearchingEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseFileSearchCallCompletedEvent))]
pub struct ResponseFileSearchCallCompletedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseWebSearchCallInProgressEvent))]
pub struct ResponseWebSearchCallInProgressEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseWebSearchCallSearchingEvent))]
pub struct ResponseWebSearchCallSearchingEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseWebSearchCallCompletedEvent))]
pub struct ResponseWebSearchCallCompletedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseReasoningSummaryPartAddedEvent))]
pub struct ResponseReasoningSummaryPartAddedEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub summary_index: u32,
    pub part: SummaryPart,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseReasoningSummaryPartDoneEvent))]
pub struct ResponseReasoningSummaryPartDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub summary_index: u32,
    pub part: SummaryPart,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseReasoningSummaryTextDeltaEvent))]
pub struct ResponseReasoningSummaryTextDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub summary_index: u32,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseReasoningSummaryTextDoneEvent))]
pub struct ResponseReasoningSummaryTextDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub summary_index: u32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseReasoningTextDeltaEvent))]
pub struct ResponseReasoningTextDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseReasoningTextDoneEvent))]
pub struct ResponseReasoningTextDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: u32,
    pub content_index: u32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseImageGenCallCompletedEvent))]
pub struct ResponseImageGenCallCompletedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseImageGenCallGeneratingEvent))]
pub struct ResponseImageGenCallGeneratingEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseImageGenCallInProgressEvent))]
pub struct ResponseImageGenCallInProgressEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseImageGenCallPartialImageEvent))]
pub struct ResponseImageGenCallPartialImageEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub partial_image_index: u32,
    pub partial_image_b64: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseMCPCallArgumentsDeltaEvent))]
pub struct ResponseMCPCallArgumentsDeltaEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseMCPCallArgumentsDoneEvent))]
pub struct ResponseMCPCallArgumentsDoneEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseMCPCallCompletedEvent))]
pub struct ResponseMCPCallCompletedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseMCPCallFailedEvent))]
pub struct ResponseMCPCallFailedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseMCPCallInProgressEvent))]
pub struct ResponseMCPCallInProgressEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseMCPListToolsCompletedEvent))]
pub struct ResponseMCPListToolsCompletedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseMCPListToolsFailedEvent))]
pub struct ResponseMCPListToolsFailedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseMCPListToolsInProgressEvent))]
pub struct ResponseMCPListToolsInProgressEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseCodeInterpreterCallInProgressEvent))]
pub struct ResponseCodeInterpreterCallInProgressEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseCodeInterpreterCallInterpretingEvent))]
pub struct ResponseCodeInterpreterCallInterpretingEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseCodeInterpreterCallCompletedEvent))]
pub struct ResponseCodeInterpreterCallCompletedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseCodeInterpreterCallCodeDeltaEvent))]
pub struct ResponseCodeInterpreterCallCodeDeltaEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseCodeInterpreterCallCodeDoneEvent))]
pub struct ResponseCodeInterpreterCallCodeDoneEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseOutputTextAnnotationAddedEvent))]
pub struct ResponseOutputTextAnnotationAddedEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub content_index: u32,
    pub annotation_index: u32,
    pub item_id: String,
    pub annotation: Value,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseQueuedEvent))]
pub struct ResponseQueuedEvent {
    pub sequence_number: u64,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseCustomToolCallInputDeltaEvent))]
pub struct ResponseCustomToolCallInputDeltaEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseCustomToolCallInputDoneEvent))]
pub struct ResponseCustomToolCallInputDoneEvent {
    pub sequence_number: u64,
    pub output_index: u32,
    pub item_id: String,
    pub input: String,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseErrorEvent))]
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
#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ResponseStreamEvent))]
pub enum ResponseStreamEvent {
    ResponseCreated(ResponseCreatedEvent),
    ResponseInProgress(ResponseInProgressEvent),
    ResponseCompleted(ResponseCompletedEvent),
    ResponseFailed(ResponseFailedEvent),
    ResponseIncomplete(ResponseIncompleteEvent),
    ResponseOutputItemAdded(ResponseOutputItemAddedEvent),
    ResponseOutputItemDone(ResponseOutputItemDoneEvent),
    ResponseContentPartAdded(ResponseContentPartAddedEvent),
    ResponseContentPartDone(ResponseContentPartDoneEvent),
    ResponseOutputTextDelta(ResponseTextDeltaEvent),
    ResponseOutputTextDone(ResponseTextDoneEvent),
    ResponseRefusalDelta(ResponseRefusalDeltaEvent),
    ResponseRefusalDone(ResponseRefusalDoneEvent),
    ResponseFunctionCallArgumentsDelta(ResponseFunctionCallArgumentsDeltaEvent),
    ResponseFunctionCallArgumentsDone(ResponseFunctionCallArgumentsDoneEvent),
    ResponseFileSearchCallInProgress(ResponseFileSearchCallInProgressEvent),
    ResponseFileSearchCallSearching(ResponseFileSearchCallSearchingEvent),
    ResponseFileSearchCallCompleted(ResponseFileSearchCallCompletedEvent),
    ResponseWebSearchCallInProgress(ResponseWebSearchCallInProgressEvent),
    ResponseWebSearchCallSearching(ResponseWebSearchCallSearchingEvent),
    ResponseWebSearchCallCompleted(ResponseWebSearchCallCompletedEvent),
    ResponseReasoningSummaryPartAdded(ResponseReasoningSummaryPartAddedEvent),
    ResponseReasoningSummaryPartDone(ResponseReasoningSummaryPartDoneEvent),
    ResponseReasoningSummaryTextDelta(ResponseReasoningSummaryTextDeltaEvent),
    ResponseReasoningSummaryTextDone(ResponseReasoningSummaryTextDoneEvent),
    ResponseReasoningTextDelta(ResponseReasoningTextDeltaEvent),
    ResponseReasoningTextDone(ResponseReasoningTextDoneEvent),
    ResponseImageGenerationCallCompleted(ResponseImageGenCallCompletedEvent),
    ResponseImageGenerationCallGenerating(ResponseImageGenCallGeneratingEvent),
    ResponseImageGenerationCallInProgress(ResponseImageGenCallInProgressEvent),
    ResponseImageGenerationCallPartialImage(ResponseImageGenCallPartialImageEvent),
    ResponseMCPCallArgumentsDelta(ResponseMCPCallArgumentsDeltaEvent),
    ResponseMCPCallArgumentsDone(ResponseMCPCallArgumentsDoneEvent),
    ResponseMCPCallCompleted(ResponseMCPCallCompletedEvent),
    ResponseMCPCallFailed(ResponseMCPCallFailedEvent),
    ResponseMCPCallInProgress(ResponseMCPCallInProgressEvent),
    ResponseMCPListToolsCompleted(ResponseMCPListToolsCompletedEvent),
    ResponseMCPListToolsFailed(ResponseMCPListToolsFailedEvent),
    ResponseMCPListToolsInProgress(ResponseMCPListToolsInProgressEvent),
    ResponseCodeInterpreterCallInProgress(ResponseCodeInterpreterCallInProgressEvent),
    ResponseCodeInterpreterCallInterpreting(ResponseCodeInterpreterCallInterpretingEvent),
    ResponseCodeInterpreterCallCompleted(ResponseCodeInterpreterCallCompletedEvent),
    ResponseCodeInterpreterCallCodeDelta(ResponseCodeInterpreterCallCodeDeltaEvent),
    ResponseCodeInterpreterCallCodeDone(ResponseCodeInterpreterCallCodeDoneEvent),
    ResponseOutputTextAnnotationAdded(ResponseOutputTextAnnotationAddedEvent),
    ResponseQueued(ResponseQueuedEvent),
    ResponseCustomToolCallInputDelta(ResponseCustomToolCallInputDeltaEvent),
    ResponseCustomToolCallInputDone(ResponseCustomToolCallInputDoneEvent),
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

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, StructuralConvert)]
#[convert(from(openai::ResponseStreamOptions))]
pub struct ResponseStreamOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_obfuscation: Option<bool>,
}
