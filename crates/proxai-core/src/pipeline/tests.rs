use std::collections::BTreeMap;

use futures_util::{StreamExt, stream};
use serde_json::json;

use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::provider::{ProviderBehavior, ProviderCompatibility};
use crate::routing::{DefaultProviderNames, ModelMatchKind, RouteRule, RoutingConfig};
use crate::translation::stream::{StreamEnd, StreamEvent, StreamTranslationInput};

use super::{Pipeline, ResponsePipeline};

fn chat_pipeline() -> Pipeline {
    Pipeline::build(
        RoutingConfig {
            default_provider_names: DefaultProviderNames {
                openai_responses: "chat".to_string(),
                openai_chat_completions: "chat".to_string(),
                anthropic_messages: "chat".to_string(),
            },
            routes: vec![RouteRule {
                name: Some("mini".to_string()),
                request_protocol: None,
                match_kind: ModelMatchKind::Exact,
                model_pattern: "gpt-client".to_string(),
                provider: "chat".to_string(),
                upstream_model: Some("gpt-upstream".to_string()),
            }],
        },
        BTreeMap::from([(
            "chat".to_string(),
            ProviderBehavior::new(
                ProviderProtocol::OpenaiChatCompletions,
                ProviderCompatibility::Compatible,
            ),
        )]),
    )
    .unwrap()
}

#[test]
fn prepares_routed_provider_request_and_response_pipeline() {
    let prepared = chat_pipeline()
        .prepare_request(
            RequestProtocol::OpenaiChatCompletions,
            json!({
                "model": "gpt-client",
                "messages": [{"role": "user", "content": "hello"}]
            }),
        )
        .unwrap();

    assert_eq!(prepared.inbound.model(), "gpt-client");
    assert_eq!(prepared.provider.name(), "chat");
    assert_eq!(prepared.provider.route_name().as_deref(), Some("mini"));
    assert_eq!(prepared.provider.upstream_model(), "gpt-upstream");
    assert_eq!(prepared.provider_payload["model"], "gpt-upstream");
    assert!(prepared.response.requires_structured_processing());
}

#[tokio::test]
async fn response_pipeline_normalizes_events_before_identity_translation() {
    let pipeline = ResponsePipeline::new(
        RequestProtocol::OpenaiChatCompletions,
        ProviderBehavior::new(
            ProviderProtocol::OpenaiChatCompletions,
            ProviderCompatibility::Compatible,
        ),
    );
    let input = stream::iter([
        Ok(StreamTranslationInput::Event(StreamEvent::new(
            "message",
            json!({
                "id": "chatcmpl_1",
                "object": "chat.completion.chunk",
                "created": 1,
                "model": "gpt-test",
                "choices": [{"index": 0, "delta": {"content": "hi"}}]
            }),
        ))),
        Ok(StreamTranslationInput::End(StreamEnd::Done)),
    ]);

    let output = pipeline
        .translate_stream(input)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(output[0].data["choices"][0]["finish_reason"], json!(null));
    assert!(output[1].is_done_sentinel());
}

#[test]
fn strict_identity_response_can_preserve_raw_carrier_bytes() {
    let pipeline = ResponsePipeline::new(
        RequestProtocol::OpenaiResponses,
        ProviderBehavior::new(
            ProviderProtocol::OpenaiResponses,
            ProviderCompatibility::Strict,
        ),
    );

    assert!(!pipeline.requires_structured_processing());
}
