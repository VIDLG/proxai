mod anthropic_messages;
mod openai_chat_completions;
mod openai_responses;

use crate::provider::ForwardedRequestView;

pub(crate) fn render_projection_compact(forwarded_request: &ForwardedRequestView<'_>) -> String {
    match forwarded_request {
        ForwardedRequestView::OpenaiResponses {
            projection,
            summary: _,
        } => openai_responses::render_projection_compact(projection),
        ForwardedRequestView::OpenaiChatCompletions {
            projection,
            summary: _,
        } => openai_chat_completions::render_projection_compact(projection),
        ForwardedRequestView::AnthropicMessages {
            projection,
            summary: _,
        } => anthropic_messages::render_projection_compact(projection),
    }
}

pub(crate) fn render_summary_compact(forwarded_request: &ForwardedRequestView<'_>) -> Vec<String> {
    match forwarded_request {
        ForwardedRequestView::OpenaiResponses {
            projection: _,
            summary,
        } => openai_responses::render_summary_compact(summary),
        ForwardedRequestView::OpenaiChatCompletions {
            projection: _,
            summary,
        } => openai_chat_completions::render_summary_compact(summary),
        ForwardedRequestView::AnthropicMessages {
            projection,
            summary,
        } => anthropic_messages::render_summary_compact(projection, summary),
    }
}

pub(super) fn render_tool_inventory<'a>(
    items: impl Iterator<Item = (&'static str, usize, &'a [String])>,
) -> Vec<String> {
    let rendered = items
        .map(|(category, count, names)| {
            if names.is_empty() {
                if count == 1 {
                    category.to_string()
                } else {
                    format!("{category}:{count}")
                }
            } else {
                format!(
                    "{category}:{count}({})",
                    names
                        .iter()
                        .map(|name| super::compact_tool_call_name(name))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
        })
        .collect::<Vec<_>>();
    if rendered.is_empty() {
        Vec::new()
    } else {
        vec![format!("tools[{}]", rendered.join(";"))]
    }
}

#[cfg(test)]
#[path = "request_hints_tests.rs"]
mod tests;
