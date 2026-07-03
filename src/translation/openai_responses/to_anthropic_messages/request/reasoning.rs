use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;

pub(super) fn output_config(
    reasoning: Option<&responses::Reasoning>,
) -> Option<anthropic::OutputConfig> {
    match reasoning?.effort? {
        responses::ReasoningEffort::Low => Some(output_effort(anthropic::OutputEffort::Low)),
        responses::ReasoningEffort::Medium => Some(output_effort(anthropic::OutputEffort::Medium)),
        responses::ReasoningEffort::High => Some(output_effort(anthropic::OutputEffort::High)),
        responses::ReasoningEffort::Xhigh => Some(output_effort(anthropic::OutputEffort::Xhigh)),
        responses::ReasoningEffort::None | responses::ReasoningEffort::Minimal => None,
    }
}

pub(super) fn thinking_config(
    reasoning: Option<&responses::Reasoning>,
) -> Option<anthropic::ThinkingConfigParam> {
    let reasoning = reasoning?;
    let has_summary = reasoning.summary.is_some();

    match reasoning.effort {
        Some(responses::ReasoningEffort::None | responses::ReasoningEffort::Minimal) => Some(
            anthropic::ThinkingConfigParam::Disabled(anthropic::ThinkingConfigDisabled),
        ),
        Some(
            responses::ReasoningEffort::Low
            | responses::ReasoningEffort::Medium
            | responses::ReasoningEffort::High
            | responses::ReasoningEffort::Xhigh,
        )
        | None
            if has_summary =>
        {
            Some(anthropic::ThinkingConfigParam::Adaptive(
                anthropic::ThinkingConfigAdaptive {
                    display: Some(anthropic::ThinkingDisplay::Summarized),
                },
            ))
        }
        _ => None,
    }
}

fn output_effort(effort: anthropic::OutputEffort) -> anthropic::OutputConfig {
    anthropic::OutputConfig {
        effort: Some(effort),
        format: None,
    }
}
