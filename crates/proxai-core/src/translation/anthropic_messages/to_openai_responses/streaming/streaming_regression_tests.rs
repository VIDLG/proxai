use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::Translator;
use crate::translation::test_support::{parse_rendered_events, translate_sse_fixture};

// Trigger: an Anthropic-compatible upstream exhausted the model context window
// before emitting any content block, then sent the Beta-defined
// `model_context_window_exceeded` terminal delta with zero output tokens.
// Symptom: Anthropic -> Responses streaming rejected the otherwise valid empty
// terminal as having no representable content. Provenance: diagnostics
// 1784598362-1784598351961 and 1784601174-1784601161991 on 2026-07-21; the
// terminal event is preserved, while message identity and lifecycle scaffolding
// were synthesized and contain no prompt data.
#[tokio::test]
async fn regression_anthropic_context_limit_empty_stream_rejected() {
    let fixture = include_str!(
        "../../../../../../../tests/fixtures/regression/anthropic-model-context-window-exceeded-empty-stream.sse"
    );
    let body = translate_sse_fixture(
        fixture,
        Translator::new(
            RequestProtocol::OpenaiResponses,
            ProviderProtocol::AnthropicMessages,
        ),
    )
    .await;

    let terminal = parse_rendered_events(&body)
        .into_iter()
        .find(|event| event.data["type"] == "response.failed")
        .expect("context exhaustion should produce a terminal failed response");

    assert_eq!(terminal.data["response"]["status"], "failed");
    assert_eq!(terminal.data["response"]["output"], serde_json::json!([]));
    assert!(terminal.data["response"]["incomplete_details"].is_null());
    assert_eq!(terminal.data["response"]["error"]["code"], "invalid_prompt");
    assert_eq!(
        terminal.data["response"]["error"]["message"],
        "model context window exceeded before generation started"
    );
    assert!(!body.contains("stream translation error"), "{body}");
}
