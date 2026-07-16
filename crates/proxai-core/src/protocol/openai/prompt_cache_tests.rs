use serde_json::json;

use crate::protocol::openai::chat_completions::request::wire::ChatCompletionRequestMessageContentPartText;
use crate::protocol::openai::responses::InputContent;

use super::{PromptCacheBreakpointConfig, PromptCacheBreakpointMode, PromptCacheBreakpointParam};

#[test]
fn prompt_cache_breakpoint_is_optional_and_non_nullable() {
    let omitted = serde_json::from_value::<ChatCompletionRequestMessageContentPartText>(json!({
        "text": "cacheable prefix"
    }))
    .unwrap();
    assert_eq!(omitted.prompt_cache_breakpoint, None);
    assert_eq!(
        serde_json::to_value(omitted).unwrap(),
        json!({ "text": "cacheable prefix" })
    );

    let present = serde_json::from_value::<ChatCompletionRequestMessageContentPartText>(json!({
        "text": "cacheable prefix",
        "prompt_cache_breakpoint": { "mode": "explicit" }
    }))
    .unwrap();
    assert_eq!(
        present.prompt_cache_breakpoint,
        Some(PromptCacheBreakpointParam {
            mode: PromptCacheBreakpointMode::Explicit,
        })
    );

    assert!(
        serde_json::from_value::<ChatCompletionRequestMessageContentPartText>(json!({
            "text": "cacheable prefix",
            "prompt_cache_breakpoint": null
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<ChatCompletionRequestMessageContentPartText>(json!({
            "text": "cacheable prefix",
            "prompt_cache_breakpoint": { "mode": "implicit" }
        }))
        .is_err()
    );

    let responses = serde_json::from_value::<InputContent>(json!({
        "type": "input_text",
        "text": "cacheable prefix",
        "prompt_cache_breakpoint": { "mode": "explicit" }
    }))
    .unwrap();
    let InputContent::InputText(responses) = responses else {
        panic!("expected Responses input_text content");
    };
    assert_eq!(
        responses.prompt_cache_breakpoint,
        Some(PromptCacheBreakpointConfig {
            mode: PromptCacheBreakpointMode::Explicit,
        })
    );

    assert!(
        serde_json::from_value::<InputContent>(json!({
            "type": "input_text",
            "text": "cacheable prefix",
            "prompt_cache_breakpoint": null
        }))
        .is_err()
    );
}
