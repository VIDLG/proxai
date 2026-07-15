use proxai_core::provider::prepare_provider_request;
use serde_json::json;

use super::*;
use crate::observe::{CaptureController, ObserveContext};
use crate::protocol::ProviderProtocol;

fn test_obs() -> ObserveContext {
    let request_id = crate::request::RequestId::from(1);
    ObserveContext::new(
        request_id,
        std::time::Instant::now(),
        CaptureController::new(None, crate::config::CaptureConfig::default()).session(request_id),
        tracing::Span::none(),
    )
}

// Trigger: Zed 1.9.0 replayed Responses history with output-only fields on
// `message` and `reasoning` input items. The forwarded provider request had
// fragments equivalent to:
// `input[1] = {"type":"message","role":"assistant","status":"completed",...}`
// `input[3] = {"type":"reasoning","id":"rs_...","status":"completed","content":[...],"summary":[],...}`
// Observed symptoms from an OpenAI-compatible upstream, as each offending field
// was exposed in sequence:
// `{"code":"unknown_parameter","message":"Unknown parameter: 'input[3].status'.","param":"input[3].status"}`
// `{"code":"array_above_max_length","message":"Invalid 'input[3].content': array too long. Expected an array with maximum length 0, but got an array with length 1 instead.","param":"input[3].content"}`
// Provenance: Zed/proxai dogfooding on 2026-07-04; inline payload is minimized
// and sanitized (private prompt/tool content redacted), so no external capture
// file is required to understand the regression.
#[test]
fn regression_zed_reasoning_output_fields_rejected_by_strict_responses_upstream() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [
            {
                "type": "message",
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": "redacted previous answer"}]
            },
            {
                "type": "reasoning",
                "id": "rs_redacted",
                "status": "completed",
                "content": [{"type": "reasoning_text", "text": "redacted reasoning"}],
                "summary": []
            }
        ]
    });

    let obs = test_obs();
    let provider_payload = prepare_provider_request(
        ProviderProtocol::OpenaiResponses,
        payload,
        "gpt-upstream",
        &obs,
    );
    let request = assemble_request(ProviderProtocol::OpenaiResponses, provider_payload, &obs)
        .expect("assemble provider request");
    let body = serde_json::from_slice::<serde_json::Value>(request.body()).unwrap();

    assert_eq!(body["model"], "gpt-upstream");
    assert!(body["input"][0].get("status").is_none());
    assert!(body["input"][1].get("status").is_none());
    assert!(body["input"][1].get("content").is_none());
    assert_eq!(body["input"][1]["summary"], json!([]));
    assert_eq!(request.capture_payload(), &body);
}
