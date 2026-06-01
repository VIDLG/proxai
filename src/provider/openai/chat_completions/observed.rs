use crate::protocol::openai::chat_completions::{
    ChatResponseProjection, ChatStreamResponseProjection, CompletionUsage, ServiceTier,
};

use super::summary::ChatResponseSummary;

#[derive(Debug, Clone)]
pub(crate) enum ObservedChatResponse {
    Response(ChatResponseProjection),
    Stream(ChatStreamResponseProjection),
}

impl Default for ObservedChatResponse {
    fn default() -> Self {
        Self::Response(ChatResponseProjection::default())
    }
}

impl ObservedChatResponse {
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Response(projection) => &projection.id,
            Self::Stream(projection) => &projection.id,
        }
    }

    pub(crate) fn model(&self) -> &str {
        match self {
            Self::Response(projection) => &projection.model,
            Self::Stream(projection) => &projection.model,
        }
    }

    pub(crate) fn service_tier(&self) -> Option<ServiceTier> {
        match self {
            Self::Response(projection) => projection.service_tier,
            Self::Stream(projection) => projection.service_tier,
        }
    }

    pub(crate) fn usage(&self) -> Option<&CompletionUsage> {
        match self {
            Self::Response(projection) => projection.usage.as_ref(),
            Self::Stream(projection) => projection.usage.as_ref(),
        }
    }

    pub(crate) fn is_done(&self) -> bool {
        match self {
            Self::Response(projection) => projection
                .choices
                .iter()
                .any(|choice| choice.finish_reason.is_some()),
            Self::Stream(projection) => projection
                .choices
                .iter()
                .any(|choice| choice.finish_reason.is_some()),
        }
    }

    fn summary(&self) -> ChatResponseSummary {
        match self {
            Self::Response(projection) => ChatResponseSummary::from(projection),
            Self::Stream(projection) => ChatResponseSummary::from(projection),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ObservedChatResponseState {
    pub(crate) latest: Option<ObservedChatResponse>,
    pub(crate) summary: ChatResponseSummary,
    pub(crate) done: bool,
}

impl ObservedChatResponseState {
    pub(crate) fn record(&mut self, response: ObservedChatResponse) {
        self.summary.merge(&response.summary());
        self.done |= response.is_done();
        self.latest = Some(response);
    }

    pub(crate) fn effective_summary(&self) -> ChatResponseSummary {
        self.summary.clone()
    }
}
