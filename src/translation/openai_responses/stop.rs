use crate::protocol::openai_responses as responses;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResponsesStopKind {
    EndTurn,
    MaxTokens,
    ToolUse,
    Refusal,
}

pub(crate) fn infer_response_stop_kind(
    response: &responses::Response,
) -> Option<ResponsesStopKind> {
    if response_has_refusal(response) {
        return Some(ResponsesStopKind::Refusal);
    }
    if response_has_tool_call(response) {
        return Some(ResponsesStopKind::ToolUse);
    }
    if let Some(details) = response.incomplete_details.as_ref() {
        return response_incomplete_stop_kind(details);
    }

    match response.status {
        responses::Status::Completed => Some(ResponsesStopKind::EndTurn),
        responses::Status::Incomplete => {
            tracing::trace!(
                reason = "Responses response is incomplete without incomplete_details.reason; treating as max_tokens"
            );
            Some(ResponsesStopKind::MaxTokens)
        }
        responses::Status::Failed
        | responses::Status::Cancelled
        | responses::Status::Queued
        | responses::Status::InProgress => None,
    }
}

fn response_has_refusal(response: &responses::Response) -> bool {
    response.output.iter().any(|item| match item {
        responses::OutputItem::Message(message) => message
            .content
            .iter()
            .any(|content| matches!(content, responses::OutputMessageContent::Refusal(_))),
        _ => false,
    })
}

fn response_has_tool_call(response: &responses::Response) -> bool {
    response.output.iter().any(|item| {
        matches!(
            item,
            responses::OutputItem::FunctionCall(_) | responses::OutputItem::CustomToolCall(_)
        )
    })
}

fn response_incomplete_stop_kind(
    details: &responses::IncompleteDetails,
) -> Option<ResponsesStopKind> {
    let reason = details.reason.as_str();
    match reason {
        "max_output_tokens" => Some(ResponsesStopKind::MaxTokens),
        "content_filter" => Some(ResponsesStopKind::Refusal),
        _ => {
            tracing::trace!(
                reason,
                "Responses incomplete_details.reason has no target stop-reason representation"
            );
            None
        }
    }
}
