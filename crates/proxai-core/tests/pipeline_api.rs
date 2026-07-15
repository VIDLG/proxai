use std::collections::BTreeMap;

use proxai_core::pipeline::Pipeline;
use proxai_core::protocol::{ProviderProtocol, RequestProtocol};
use proxai_core::provider::{ProviderBehavior, ProviderCompatibility};
use proxai_core::routing::{DefaultProviderNames, RoutingConfig};
use serde_json::json;

#[test]
fn prepares_an_exchange_through_the_public_pipeline_api() {
    let pipeline = Pipeline::build(
        RoutingConfig {
            default_provider_names: DefaultProviderNames {
                openai_responses: "openai".to_string(),
                openai_chat_completions: "openai".to_string(),
                anthropic_messages: "openai".to_string(),
            },
            routes: Vec::new(),
        },
        BTreeMap::from([(
            "openai".to_string(),
            ProviderBehavior::new(
                ProviderProtocol::OpenaiResponses,
                ProviderCompatibility::Strict,
            ),
        )]),
    )
    .unwrap();

    let prepared = pipeline
        .prepare_request(
            RequestProtocol::OpenaiResponses,
            json!({"model": "gpt-test", "input": "hello"}),
        )
        .unwrap();

    assert_eq!(prepared.provider.name(), "openai");
    assert_eq!(prepared.provider_payload["model"], "gpt-test");
    assert!(!prepared.response.requires_structured_processing());
}
