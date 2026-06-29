use serde_json::json;

use crate::provider::ProviderRequestView;
use crate::provider::anthropic_messages::request::RequestSummary;
use crate::provider::openai::responses::RequestSummary as ResponsesRequestSummary;

#[test]
fn openai_responses_include_hints_use_readable_names() {
    let projection = serde_json::from_value(json!({
        "model": "gpt-test",
        "include": ["reasoning.encrypted_content"],
        "input": "hello"
    }))
    .unwrap();
    let summary = ResponsesRequestSummary::from(&projection);
    let view = ProviderRequestView::OpenaiResponses {
        projection: &projection,
        summary: &summary,
    };

    let projection_hint = super::render_projection_compact(&view);

    assert!(projection_hint.contains("include[rsn.enc]"));
    assert!(!projection_hint.contains("inc["));
    assert!(!projection_hint.contains("reasoning.enc"));
}
#[test]
fn anthropic_hints_render_display_tokens_from_projection() {
    let projection = serde_json::from_value(json!({
        "model": "claude-request",
        "max_tokens": 256,
        "stream": true,
        "service_tier": "standard_only",
        "thinking": {"type": "adaptive", "display": "summarized"},
        "tool_choice": {"type": "tool", "name": "lookup"},
        "tools": [{
            "type": "custom",
            "name": "lookup",
            "description": "Look up a record",
            "input_schema": {"type": "object", "properties": {}, "required": []}
        }],
        "messages": [{"role": "user", "content": "hello"}]
    }))
    .unwrap();
    let summary = RequestSummary::from(&projection);
    let view = ProviderRequestView::AnthropicMessages {
        projection: &projection,
        summary: &summary,
    };

    let projection_hint = super::render_projection_compact(&view);
    let summary_hints = super::render_summary_compact(&view);

    assert!(projection_hint.contains("tier:standard_only"));
    assert!(summary_hints.contains(&"tc:tool:lookup".to_string()));
    assert!(summary_hints.contains(&"think:adaptive".to_string()));
    assert!(summary_hints.contains(&"tools[c:1(lookup)]".to_string()));
}
