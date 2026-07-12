//! Streaming outbound helpers for target OpenAI Responses.
//!
//! These helpers own target-protocol stream event construction that is shared
//! by translation pairs which emit Responses streams. Pair-local modules should
//! still build events that depend on source-specific accumulated state.

use crate::protocol::openai_responses as responses;

pub(crate) fn response_created(
    sequence_number: u64,
    response: responses::Response,
) -> responses::ResponseStreamEvent {
    responses::ResponseStreamEvent::ResponseCreated(responses::ResponseCreatedEvent {
        sequence_number,
        response,
    })
}

pub(crate) fn response_terminal(
    sequence_number: u64,
    response: responses::Response,
    status: responses::Status,
) -> responses::ResponseStreamEvent {
    match status {
        responses::Status::Incomplete => {
            responses::ResponseStreamEvent::ResponseIncomplete(responses::ResponseIncompleteEvent {
                sequence_number,
                response,
            })
        }
        _ => responses::ResponseStreamEvent::ResponseCompleted(responses::ResponseCompletedEvent {
            sequence_number,
            response,
        }),
    }
}

pub(crate) fn output_item_added(
    sequence_number: u64,
    output_index: u32,
    item: responses::OutputItem,
) -> responses::ResponseStreamEvent {
    responses::ResponseStreamEvent::ResponseOutputItemAdded(
        responses::ResponseOutputItemAddedEvent {
            sequence_number,
            output_index,
            item,
        },
    )
}

pub(crate) fn output_item_done(
    sequence_number: u64,
    output_index: u32,
    item: responses::OutputItem,
) -> responses::ResponseStreamEvent {
    responses::ResponseStreamEvent::ResponseOutputItemDone(responses::ResponseOutputItemDoneEvent {
        sequence_number,
        output_index,
        item,
    })
}

pub(crate) fn output_text_done(
    sequence_number: u64,
    item_id: String,
    output_index: u32,
    text: String,
) -> responses::ResponseStreamEvent {
    responses::ResponseStreamEvent::ResponseOutputTextDone(responses::ResponseTextDoneEvent {
        sequence_number,
        item_id,
        output_index,
        content_index: 0,
        text,
        logprobs: Vec::new(),
    })
}

pub(crate) fn reasoning_text_done(
    sequence_number: u64,
    item_id: String,
    output_index: u32,
    text: String,
) -> responses::ResponseStreamEvent {
    responses::ResponseStreamEvent::ResponseReasoningTextDone(
        responses::ResponseReasoningTextDoneEvent {
            sequence_number,
            item_id,
            output_index,
            content_index: 0,
            text,
        },
    )
}

pub(crate) fn tool_arguments_done(
    sequence_number: u64,
    item_id: String,
    output_index: u32,
    name: String,
    arguments: String,
) -> responses::ResponseStreamEvent {
    responses::ResponseStreamEvent::ResponseFunctionCallArgumentsDone(
        responses::ResponseFunctionCallArgumentsDoneEvent {
            sequence_number,
            item_id,
            output_index,
            name,
            arguments,
        },
    )
}

pub(crate) fn output_text_delta(
    sequence_number: u64,
    item_id: String,
    output_index: u32,
    delta: String,
) -> responses::ResponseStreamEvent {
    responses::ResponseStreamEvent::ResponseOutputTextDelta(responses::ResponseTextDeltaEvent {
        sequence_number,
        item_id,
        output_index,
        content_index: 0,
        delta,
        logprobs: Vec::new(),
    })
}

pub(crate) fn refusal_delta(
    sequence_number: u64,
    item_id: String,
    output_index: u32,
    delta: String,
) -> responses::ResponseStreamEvent {
    responses::ResponseStreamEvent::ResponseRefusalDelta(responses::ResponseRefusalDeltaEvent {
        sequence_number,
        item_id,
        output_index,
        content_index: 0,
        delta,
    })
}

pub(crate) fn refusal_done(
    sequence_number: u64,
    item_id: String,
    output_index: u32,
    refusal: String,
) -> responses::ResponseStreamEvent {
    responses::ResponseStreamEvent::ResponseRefusalDone(responses::ResponseRefusalDoneEvent {
        sequence_number,
        item_id,
        output_index,
        content_index: 0,
        refusal,
    })
}

pub(crate) fn reasoning_text_delta(
    sequence_number: u64,
    item_id: String,
    output_index: u32,
    delta: String,
) -> responses::ResponseStreamEvent {
    responses::ResponseStreamEvent::ResponseReasoningTextDelta(
        responses::ResponseReasoningTextDeltaEvent {
            sequence_number,
            item_id,
            output_index,
            content_index: 0,
            delta,
        },
    )
}

pub(crate) fn tool_arguments_delta(
    sequence_number: u64,
    item_id: String,
    output_index: u32,
    delta: String,
) -> responses::ResponseStreamEvent {
    responses::ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(
        responses::ResponseFunctionCallArgumentsDeltaEvent {
            sequence_number,
            item_id,
            output_index,
            delta,
        },
    )
}
