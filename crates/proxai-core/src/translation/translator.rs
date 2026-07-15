use std::sync::Arc;

use async_stream::stream;
use delegate::delegate;
use futures_util::{Stream, StreamExt};
use serde_json::Value;

use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::anthropic_messages::to_openai_chat_completions::AnthropicToChatStreaming;
use crate::translation::anthropic_messages::to_openai_responses::AnthropicToResponsesStreaming;
use crate::translation::openai_chat_completions::to_anthropic_messages::ChatToAnthropicStreaming;
use crate::translation::openai_chat_completions::to_openai_responses::ChatToResponsesStreaming;
use crate::translation::openai_responses::to_anthropic_messages::ResponsesToAnthropicStreaming;
use crate::translation::openai_responses::to_openai_chat_completions::ResponsesToChatStreaming;

use super::context::{
    NoopTranslationObserver, TranslationContext, TranslationObserver, TranslationPhase,
    TranslationRoute, TranslationScope,
};
use super::stream::{
    StreamEnd, StreamEvent, StreamEventStream, StreamTranslationInput, StreamTranslationResult,
};
use super::{TranslationResult, translate_non_streaming_response, translate_request};

enum PairStreamingState {
    Identity,
    AnthropicToChat(Box<AnthropicToChatStreaming>),
    AnthropicToResponses(Box<AnthropicToResponsesStreaming>),
    ChatToAnthropic(Box<ChatToAnthropicStreaming>),
    ChatToResponses(Box<ChatToResponsesStreaming>),
    ResponsesToAnthropic(Box<ResponsesToAnthropicStreaming>),
    ResponsesToChat(Box<ResponsesToChatStreaming>),
}

impl PairStreamingState {
    fn new(route: TranslationRoute) -> Self {
        match (route.request_protocol, route.provider_protocol) {
            (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiChatCompletions) => {
                Self::ChatToResponses(Box::default())
            }
            (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiResponses) => {
                Self::ResponsesToChat(Box::default())
            }
            (RequestProtocol::OpenaiResponses, ProviderProtocol::AnthropicMessages) => {
                Self::AnthropicToResponses(Box::default())
            }
            (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::AnthropicMessages) => {
                Self::AnthropicToChat(Box::default())
            }
            (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiResponses) => {
                Self::ResponsesToAnthropic(Box::default())
            }
            (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiChatCompletions) => {
                Self::ChatToAnthropic(Box::default())
            }
            (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiResponses)
            | (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiChatCompletions)
            | (RequestProtocol::AnthropicMessages, ProviderProtocol::AnthropicMessages) => {
                Self::Identity
            }
        }
    }

    fn translate_event(
        &mut self,
        event: StreamEvent,
        scope: &TranslationScope,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        match self {
            Self::Identity => Ok(vec![event]),
            Self::AnthropicToChat(state) => state.translate_event(event, scope),
            Self::AnthropicToResponses(state) => state.translate_event(event, scope),
            Self::ChatToAnthropic(state) => state.translate_event(event, scope),
            Self::ChatToResponses(state) => state.translate_event(event, scope),
            Self::ResponsesToAnthropic(state) => state.translate_event(event, scope),
            Self::ResponsesToChat(state) => state.translate_event(event, scope),
        }
    }

    fn finish_stream(&mut self, end: StreamEnd) -> StreamTranslationResult<Vec<StreamEvent>> {
        match self {
            Self::Identity => Ok(Vec::new()),
            Self::AnthropicToChat(state) => state.finish_stream(end),
            Self::AnthropicToResponses(state) => state.finish_stream(end),
            Self::ChatToAnthropic(state) => state.finish_stream(end),
            Self::ChatToResponses(state) => state.finish_stream(end),
            Self::ResponsesToAnthropic(state) => state.finish_stream(end),
            Self::ResponsesToChat(state) => state.finish_stream(end),
        }
    }

    fn translate_stream<S>(input: S, scope: TranslationScope) -> StreamEventStream
    where
        S: Stream<Item = StreamTranslationResult<StreamTranslationInput>> + Send + 'static,
    {
        let route = scope.route();
        Box::pin(stream! {
            let mut pair = Self::new(route);
            let mut input = Box::pin(input);
            while let Some(item) = input.next().await {
                let outputs = match item {
                    Ok(StreamTranslationInput::Event(event)) => {
                        pair.translate_event(event, &scope)
                    }
                    Ok(StreamTranslationInput::End(end)) => {
                        let result = pair.finish_stream(end);
                        match result {
                            Ok(outputs) => {
                                for output in outputs {
                                    yield Ok(output);
                                }
                            }
                            Err(error) => yield Err(error),
                        }
                        return;
                    }
                    Err(error) => {
                        yield Err(error);
                        return;
                    }
                };

                match outputs {
                    Ok(outputs) => {
                        for output in outputs {
                            yield Ok(output);
                        }
                    }
                    Err(error) => {
                        yield Err(error);
                        return;
                    }
                }
            }

            match pair.finish_stream(StreamEnd::Eof) {
                Ok(outputs) => {
                    for output in outputs {
                        yield Ok(output);
                    }
                }
                Err(error) => yield Err(error),
            }
        })
    }
}

/// Configured entry point for request, response, or structured-stream translation.
///
/// Value translation borrows the translator. Stream translation consumes it
/// and creates one private protocol-pair state machine inside the returned
/// stream, so state cannot accidentally be reused across streams.
#[derive(Clone)]
pub struct Translator {
    context: TranslationContext,
}

impl Translator {
    pub fn new(request_protocol: RequestProtocol, provider_protocol: ProviderProtocol) -> Self {
        let route = TranslationRoute {
            request_protocol,
            provider_protocol,
        };
        Self {
            context: TranslationContext::new(route, Arc::new(NoopTranslationObserver)),
        }
    }

    pub fn with_observer(mut self, observer: impl TranslationObserver) -> Self {
        self.context = self.context.with_observer(Arc::new(observer));
        self
    }

    delegate! {
        to self.context {
            pub fn route(&self) -> TranslationRoute;
        }
    }

    pub fn translate_request(&self, payload: &Value) -> TranslationResult<Value> {
        translate_request(payload, &self.context.scope(TranslationPhase::Request))
    }

    pub fn translate_response(&self, payload: Value) -> TranslationResult<Value> {
        translate_non_streaming_response(
            payload,
            &self.context.scope(TranslationPhase::NonStreamingResponse),
        )
    }

    /// Translate one structured response stream.
    ///
    /// An explicit [`StreamTranslationInput::End`] preserves carrier terminal
    /// semantics such as `[DONE]`; natural input exhaustion is treated as EOF.
    pub fn translate_stream<S>(self, input: S) -> StreamEventStream
    where
        S: Stream<Item = StreamTranslationResult<StreamTranslationInput>> + Send + 'static,
    {
        PairStreamingState::translate_stream(
            input,
            self.context.scope(TranslationPhase::StreamingResponse),
        )
    }
}

#[cfg(test)]
#[path = "translator_tests.rs"]
mod tests;
