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
    if let Some(details) = response.incomplete_details.as_non_null()
        && let Some(kind) = response_incomplete_stop_kind(details)
    {
        return Some(kind);
    }

    match response.status {
        Some(responses::Status::Completed) => Some(ResponsesStopKind::EndTurn),
        Some(responses::Status::Incomplete) => {
            tracing::trace!(
                reason = "Responses response is incomplete without incomplete_details.reason; treating as max_tokens"
            );
            Some(ResponsesStopKind::MaxTokens)
        }
        Some(responses::Status::Failed)
        | Some(responses::Status::Cancelled)
        | Some(responses::Status::Queued)
        | Some(responses::Status::InProgress)
        | None => None,
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
    match details.reason {
        Some(responses::IncompleteDetailsReason::MaxOutputTokens) => {
            Some(ResponsesStopKind::MaxTokens)
        }
        Some(responses::IncompleteDetailsReason::ContentFilter) => Some(ResponsesStopKind::Refusal),
        None => None,
    }
}
