use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use crate::translation::TranslationScope;
use crate::translation::anthropic_messages::outbound::output_config;

#[derive(Default)]
pub(super) struct AnthropicReasoning {
    pub output_config: Option<anthropic::OutputConfig>,
    pub thinking: Option<anthropic::ThinkingConfigParam>,
}

pub(super) fn translate_reasoning(
    reasoning: Option<&responses::Reasoning>,
    scope: &TranslationScope,
) -> AnthropicReasoning {
    let Some(reasoning) = reasoning else {
        return AnthropicReasoning::default();
    };

    if reasoning.mode.is_some() {
        scope.dropped(
            "Responses reasoning.mode",
            "Anthropic Messages has no equivalent reasoning execution mode",
        );
    }
    if reasoning.context.is_non_null() {
        scope.dropped(
            "Responses reasoning.context",
            "Anthropic Messages has no equivalent reasoning history-selection control",
        );
    }

    let summary = reasoning_summary(reasoning, scope);
    let effort = reasoning.effort.as_non_null().copied();
    let output_config = output_effort(effort, scope).map(output_config);
    let thinking = match effort {
        Some(responses::ReasoningEffort::None) => {
            if summary.is_some() {
                scope.dropped(
                    "Responses reasoning.summary",
                    "reasoning.effort `none` disables reasoning; Anthropic cannot request a thinking summary while thinking is disabled",
                );
            }
            Some(anthropic::ThinkingConfigParam::Disabled(
                anthropic::ThinkingConfigDisabled,
            ))
        }
        Some(responses::ReasoningEffort::Minimal) if summary.is_none() => Some(
            anthropic::ThinkingConfigParam::Disabled(anthropic::ThinkingConfigDisabled),
        ),
        _ => summary.map(|summary| {
            anthropic::ThinkingConfigParam::Adaptive(anthropic::ThinkingConfigAdaptive {
                display: Some(thinking_display(summary, scope)).into(),
            })
        }),
    };

    AnthropicReasoning {
        output_config,
        thinking,
    }
}

fn reasoning_summary(
    reasoning: &responses::Reasoning,
    scope: &TranslationScope,
) -> Option<responses::ReasoningSummary> {
    if let Some(summary) = reasoning.summary.as_non_null().copied() {
        if reasoning.generate_summary.is_non_null() {
            scope.dropped(
                "Responses reasoning.generate_summary",
                "reasoning.summary takes precedence over the deprecated field",
            );
        }
        return Some(summary);
    }

    let summary = reasoning.generate_summary.as_non_null().copied()?;
    scope.adapted(
        "Responses reasoning.generate_summary",
        "used as deprecated alias for reasoning.summary",
    );
    Some(summary)
}

fn output_effort(
    effort: Option<responses::ReasoningEffort>,
    scope: &TranslationScope,
) -> Option<anthropic::OutputEffort> {
    match effort? {
        responses::ReasoningEffort::None => None,
        responses::ReasoningEffort::Minimal => {
            scope.adapted(
                "Responses reasoning.effort `minimal`",
                "Anthropic output_config.effort has no minimal level; using low",
            );
            Some(anthropic::OutputEffort::Low)
        }
        responses::ReasoningEffort::Low => Some(anthropic::OutputEffort::Low),
        responses::ReasoningEffort::Medium => Some(anthropic::OutputEffort::Medium),
        responses::ReasoningEffort::High => Some(anthropic::OutputEffort::High),
        responses::ReasoningEffort::Xhigh => Some(anthropic::OutputEffort::Xhigh),
        responses::ReasoningEffort::Max => Some(anthropic::OutputEffort::Max),
    }
}

fn thinking_display(
    summary: responses::ReasoningSummary,
    scope: &TranslationScope,
) -> anthropic::ThinkingDisplay {
    if matches!(
        summary,
        responses::ReasoningSummary::Concise | responses::ReasoningSummary::Detailed
    ) {
        scope.adapted(
            format!("Responses reasoning summary `{summary}`"),
            "Anthropic thinking display cannot distinguish concise/detailed; using summarized",
        );
    }
    anthropic::ThinkingDisplay::Summarized
}
