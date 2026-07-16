use std::collections::BTreeMap;

use futures_util::{StreamExt, stream};
use serde_json::json;

use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::provider::{ProviderBehavior, ProviderCompatibility};
use crate::routing::{DefaultProviderNames, ModelMatchKind, RouteRule, RoutingConfig};
use crate::translation::stream::{StreamEnd, StreamEvent, StreamTranslationInput};

use super::{Pipeline, ResponsePipeline};

fn pipeline_for(provider_protocol: ProviderProtocol) -> Pipeline {
    Pipeline::build(
        RoutingConfig {
            default_provider_names: DefaultProviderNames {
                openai_responses: "provider".to_string(),
                openai_chat_completions: "provider".to_string(),
                anthropic_messages: "provider".to_string(),
            },
            routes: Vec::new(),
        },
        BTreeMap::from([(
            "provider".to_string(),
            ProviderBehavior::new(provider_protocol, ProviderCompatibility::Compatible),
        )]),
    )
    .unwrap()
}

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

#[test]
fn translates_openai_responses_inbound_to_chat_provider_request() {
    let prepared = pipeline_for(ProviderProtocol::OpenaiChatCompletions)
        .prepare_request(
            RequestProtocol::OpenaiResponses,
            json!({
                "model": "glm-5.1",
                "instructions": "Be concise.",
                "input": [{
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "hello"}]
                }],
                "stream": true,
                "max_output_tokens": 64
            }),
        )
        .unwrap();
    let provider_body = prepared.provider_payload;

    assert_eq!(provider_body["model"], "glm-5.1");
    assert_eq!(provider_body["max_completion_tokens"], 64);
    assert_eq!(provider_body["stream"], true);
    assert_eq!(provider_body["messages"][0]["role"], "developer");
    assert_eq!(provider_body["messages"][0]["content"], "Be concise.");
    assert_eq!(provider_body["messages"][1]["role"], "user");
    assert_eq!(provider_body["messages"][1]["content"][0]["text"], "hello");
}

#[test]
fn translates_glm_openai_responses_inbound_to_anthropic_provider_request() {
    let prepared = pipeline_for(ProviderProtocol::AnthropicMessages)
        .prepare_request(
            RequestProtocol::OpenaiResponses,
            json!({
                "model": "glm-5.1",
                "instructions": "You are a proxai live translation smoke test. Reply briefly.",
                "input": [{
                    "type": "message",
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": "Reply with the exact text: proxai-translation-live-ok"
                    }]
                }],
                "stream": false,
                "max_output_tokens": 64
            }),
        )
        .unwrap();
    let provider_body = prepared.provider_payload;

    assert_eq!(provider_body["model"], "glm-5.1");
    assert_eq!(provider_body["max_tokens"], 64);
    assert_eq!(
        provider_body["system"],
        "You are a proxai live translation smoke test. Reply briefly."
    );
    assert_eq!(provider_body["stream"], false);
    assert_eq!(provider_body["messages"][0]["role"], "user");
    assert_eq!(
        provider_body["messages"][0]["content"][0],
        json!({
            "type": "text",
            "text": "Reply with the exact text: proxai-translation-live-ok"
        })
    );
}

#[test]
fn translates_glm_5_2_openai_responses_inbound_to_anthropic_provider_request() {
    // The translation is model-agnostic, but exercising this real client model
    // label guards the complete ingress, routing, and translation composition.
    let prepared = pipeline_for(ProviderProtocol::AnthropicMessages)
        .prepare_request(
            RequestProtocol::OpenaiResponses,
            json!({
                "model": "glm-5.2",
                "instructions": "You are a proxai live translation smoke test. Reply briefly.",
                "input": [{
                    "type": "message",
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": "Reply with the exact text: proxai-translation-live-ok"
                    }]
                }],
                "stream": false,
                "max_output_tokens": 64,
                "reasoning": {"effort": "medium", "summary": "auto"},
                "tools": [{
                    "type": "function",
                    "name": "lookup",
                    "description": "Look up a record",
                    "parameters": {
                        "type": "object",
                        "properties": {"id": {"type": "string"}},
                        "required": ["id"]
                    }
                }],
                "tool_choice": "auto",
                "parallel_tool_calls": true
            }),
        )
        .unwrap();
    let provider_body = prepared.provider_payload;

    assert_eq!(provider_body["model"], "glm-5.2");
    assert_eq!(provider_body["max_tokens"], 64);
    assert_eq!(provider_body["stream"], false);
    assert_eq!(
        provider_body["system"],
        "You are a proxai live translation smoke test. Reply briefly."
    );
    assert_eq!(provider_body["messages"][0]["role"], "user");
    assert_eq!(
        provider_body["messages"][0]["content"][0],
        json!({
            "type": "text",
            "text": "Reply with the exact text: proxai-translation-live-ok"
        })
    );
    assert_eq!(provider_body["tools"][0]["type"], "custom");
    assert_eq!(provider_body["tools"][0]["name"], "lookup");
    assert_eq!(provider_body["tool_choice"]["type"], "auto");
    assert_ne!(
        provider_body["tool_choice"]["disable_parallel_tool_use"],
        true
    );
    assert_eq!(provider_body["output_config"]["effort"], "medium");
    assert_eq!(provider_body["thinking"]["type"], "adaptive");
    assert_eq!(provider_body["thinking"]["display"], "summarized");
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
