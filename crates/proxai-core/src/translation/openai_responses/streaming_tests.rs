use serde_json::{Value, json};

use crate::translation::stream::{StreamIdentity, StreamTranslationError};

use super::{ForwardedContent, ResponsesInboundLifecycle};

fn output_item_added() -> Value {
    json!({
        "type": "response.output_item.added",
        "sequence_number": 1,
        "output_index": 0,
        "item": {
            "type": "message",
            "id": "msg_1",
            "role": "assistant",
            "status": "in_progress",
            "content": []
        }
    })
}

fn output_item_done() -> Value {
    json!({
        "type": "response.output_item.done",
        "sequence_number": 2,
        "output_index": 0,
        "item": {
            "type": "message",
            "id": "msg_1",
            "role": "assistant",
            "status": "completed",
            "content": []
        }
    })
}

fn streaming_lifecycle() -> ResponsesInboundLifecycle<()> {
    let mut lifecycle = ResponsesInboundLifecycle::default();
    lifecycle
        .ensure_response_stream(
            StreamIdentity::new("resp_1".to_string(), "test-model".to_string()),
            (),
        )
        .unwrap();
    lifecycle
}

#[test]
fn reconciles_authoritative_snapshot_suffix() {
    let mut forwarded = ForwardedContent::from("Hel".to_string());

    assert_eq!(
        forwarded.reconcile_snapshot("Hello").unwrap(),
        Some("lo".to_string())
    );
    assert_eq!(forwarded.reconcile_snapshot("Hello").unwrap(), None);
}

#[test]
fn rejects_snapshot_that_does_not_extend_forwarded_content() {
    let mut forwarded = ForwardedContent::from("Hello".to_string());

    assert!(forwarded.reconcile_snapshot("Hi").is_err());
}

#[test]
fn rejects_output_item_event_before_response_created() {
    let mut lifecycle = ResponsesInboundLifecycle::<()>::default();

    let error = lifecycle
        .parse_stream_event(output_item_added())
        .unwrap_err();

    assert!(matches!(&error, StreamTranslationError::Semantic(_)));
    assert!(error.to_string().contains(
        "Responses stream emitted response.output_item.added while lifecycle was waiting; expected streaming"
    ));
}

#[test]
fn accepts_function_output_item_without_id_until_its_delta_binds_one() {
    let mut lifecycle = streaming_lifecycle();
    lifecycle
        .parse_stream_event(json!({
            "type": "response.output_item.added",
            "sequence_number": 1,
            "output_index": 0,
            "item": {
                "type": "function_call",
                "call_id": "call_1",
                "name": "lookup",
                "arguments": "",
                "status": "in_progress"
            }
        }))
        .unwrap();

    lifecycle
        .parse_stream_event(json!({
            "type": "response.function_call_arguments.delta",
            "sequence_number": 2,
            "output_index": 0,
            "item_id": "fc_1",
            "delta": "{}"
        }))
        .unwrap();

    lifecycle
        .parse_stream_event(json!({
            "type": "response.output_item.done",
            "sequence_number": 3,
            "output_index": 0,
            "item": {
                "type": "function_call",
                "id": "fc_1",
                "call_id": "call_1",
                "name": "lookup",
                "arguments": "{}",
                "status": "completed"
            }
        }))
        .unwrap();
}

#[test]
fn rejects_reasoning_output_item_without_id() {
    let mut lifecycle = streaming_lifecycle();

    let error = lifecycle
        .parse_stream_event(json!({
            "type": "response.output_item.added",
            "sequence_number": 1,
            "output_index": 0,
            "item": {
                "type": "reasoning",
                "summary": [],
                "status": "in_progress"
            }
        }))
        .unwrap_err();

    let StreamTranslationError::JsonPayload(error) = error else {
        panic!("expected a typed JSON payload error");
    };
    assert_eq!(error.context(), "OpenAI Responses stream event");
    assert!(error.to_string().contains("missing field `id`"));
}

#[test]
fn rejects_output_item_added_with_inline_message_content() {
    let mut lifecycle = streaming_lifecycle();

    let error = lifecycle
        .parse_stream_event(json!({
            "type": "response.output_item.added",
            "sequence_number": 1,
            "output_index": 0,
            "item": {
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "status": "in_progress",
                "content": [{
                    "type": "output_text",
                    "text": "inline",
                    "annotations": [],
                    "logprobs": []
                }]
            }
        }))
        .unwrap_err();

    assert!(matches!(&error, StreamTranslationError::Semantic(_)));
    assert!(error.to_string().contains(
        "response.output_item.added with non-empty message content; content must be emitted through response.content_part.* events"
    ));
}

#[test]
fn rejects_output_item_added_with_inline_reasoning_summary() {
    let mut lifecycle = streaming_lifecycle();

    let error = lifecycle
        .parse_stream_event(json!({
            "type": "response.output_item.added",
            "sequence_number": 1,
            "output_index": 0,
            "item": {
                "type": "reasoning",
                "id": "rs_1",
                "summary": [{"type": "summary_text", "text": "inline"}],
                "status": "in_progress"
            }
        }))
        .unwrap_err();

    assert!(matches!(&error, StreamTranslationError::Semantic(_)));
    assert!(error.to_string().contains(
        "response.output_item.added with reasoning content or summary; reasoning must be emitted through response.reasoning_* events"
    ));
}

#[test]
fn rejects_delta_after_output_item_done() {
    let mut lifecycle = streaming_lifecycle();
    lifecycle.parse_stream_event(output_item_added()).unwrap();
    lifecycle.parse_stream_event(output_item_done()).unwrap();

    let error = lifecycle
        .parse_stream_event(json!({
            "type": "response.output_text.delta",
            "sequence_number": 3,
            "output_index": 0,
            "content_index": 0,
            "item_id": "msg_1",
            "delta": "late",
            "logprobs": []
        }))
        .unwrap_err();

    assert!(matches!(&error, StreamTranslationError::Semantic(_)));
    assert!(error
        .to_string()
        .contains("Responses stream emitted response.output_text.delta for output_index 0 after response.output_item.done"));
}

#[test]
fn rejects_duplicate_output_item_done() {
    let mut lifecycle = streaming_lifecycle();
    lifecycle.parse_stream_event(output_item_added()).unwrap();
    lifecycle.parse_stream_event(output_item_done()).unwrap();

    let error = lifecycle
        .parse_stream_event(output_item_done())
        .unwrap_err();

    assert!(matches!(&error, StreamTranslationError::Semantic(_)));
    assert!(error.to_string().contains(
        "Responses stream emitted duplicate response.output_item.done for output_index 0"
    ));
}
