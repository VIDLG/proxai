use serde_json::Value;

use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::Translator;
use crate::translation::test_support::translate_sse_fixture;

async fn translate_responses_stream_body(body: &'static str) -> String {
    let body = complete_response_snapshots(body);
    translate_sse_fixture(
        &body,
        Translator::new(
            RequestProtocol::OpenaiChatCompletions,
            ProviderProtocol::OpenaiResponses,
        ),
    )
    .await
}

fn complete_response_snapshots(body: &str) -> String {
    let mut completed = String::new();
    for line in body.lines() {
        if let Some(data) = line.strip_prefix("data: ")
            && let Ok(mut event) = serde_json::from_str::<Value>(data)
        {
            if let Some(response) = event.get_mut("response").and_then(Value::as_object_mut) {
                for field in [
                    "metadata",
                    "temperature",
                    "top_p",
                    "error",
                    "incomplete_details",
                    "instructions",
                ] {
                    response.entry(field.to_string()).or_insert(Value::Null);
                }
            }
            completed.push_str("data: ");
            completed.push_str(&event.to_string());
        } else {
            completed.push_str(line);
        }
        completed.push('\n');
    }
    completed
}

fn chat_stream_payloads(body: &str) -> Vec<Value> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .filter_map(|data| serde_json::from_str(data).ok())
        .collect()
}

#[tokio::test]
async fn translates_responses_text_stream_to_chat_completions_sse() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_123\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\",\"usage\":{\"input_tokens\":8,\"input_tokens_details\":{\"cached_tokens\":0},\"output_tokens\":0,\"output_tokens_details\":{\"reasoning_tokens\":0},\"total_tokens\":8}}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":3,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"hel\",\"logprobs\":[]}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":4,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"lo\",\"logprobs\":[]}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":5,\"response\":{\"id\":\"resp_123\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"completed\",\"usage\":{\"input_tokens\":8,\"input_tokens_details\":{\"cached_tokens\":0},\"output_tokens\":2,\"output_tokens_details\":{\"reasoning_tokens\":0},\"total_tokens\":10}}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("\"role\":\"assistant\""));
    assert!(text.contains("\"content\":\"hel\""));
    assert!(text.contains("\"content\":\"lo\""));
    assert!(text.contains("\"finish_reason\":\"stop\""));
    assert!(text.contains("data: [DONE]"));
}

#[tokio::test]
async fn translates_responses_tool_calls_stream_to_chat_completions_sse() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_456\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_abc\",\"name\":\"get_weather\",\"arguments\":\"\",\"status\":\"in_progress\"}}\n\n",
        "event: response.function_call_arguments.delta\n",
        "data: {\"type\":\"response.function_call_arguments.delta\",\"sequence_number\":3,\"item_id\":\"fc_1\",\"output_index\":0,\"delta\":\"{\\\"city\\\":\\\"SF\\\"}\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":4,\"response\":{\"id\":\"resp_456\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"completed\"}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("\"name\":\"get_weather\""));
    assert!(text.contains("\"id\":\"call_abc\""));
    assert!(text.contains("city"));
    assert!(text.contains("SF"));
    assert!(text.contains("\"finish_reason\":\"tool_calls\""));
}

#[tokio::test]
async fn translates_responses_incomplete_to_length_finish_reason() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_789\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":3,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"partial\",\"logprobs\":[]}\n\n",
        "event: response.incomplete\n",
        "data: {\"type\":\"response.incomplete\",\"sequence_number\":4,\"response\":{\"id\":\"resp_789\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"incomplete\"}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("\"finish_reason\":\"length\""));
}

#[tokio::test]
async fn translates_responses_refusal_to_chat_refusal_delta() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_refusal\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n",
        "event: response.refusal.delta\n",
        "data: {\"type\":\"response.refusal.delta\",\"sequence_number\":3,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"cannot comply\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":4,\"response\":{\"id\":\"resp_refusal\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"completed\"}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("\"refusal\":\"cannot comply\""));
    assert!(text.contains("\"finish_reason\":\"stop\""));
    assert!(text.contains("data: [DONE]"));
}

#[tokio::test]
async fn compacts_responses_output_index_to_chat_tool_call_index() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_tool_index\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":3,\"output_index\":1,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"lookup\",\"arguments\":\"\",\"status\":\"in_progress\"}}\n\n",
        "event: response.function_call_arguments.delta\n",
        "data: {\"type\":\"response.function_call_arguments.delta\",\"sequence_number\":4,\"item_id\":\"fc_1\",\"output_index\":1,\"delta\":\"{}\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":5,\"response\":{\"id\":\"resp_tool_index\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"completed\"}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    let chunks = chat_stream_payloads(&text);
    let tool_indexes = chunks
        .iter()
        .filter_map(|chunk| chunk["choices"][0]["delta"]["tool_calls"][0]["index"].as_u64())
        .collect::<Vec<_>>();
    assert!(!tool_indexes.is_empty());
    assert!(tool_indexes.iter().all(|index| *index == 0));
}

#[tokio::test]
async fn rejects_text_delta_with_mismatched_item_id() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_mismatch\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_expected\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":3,\"item_id\":\"msg_wrong\",\"output_index\":0,\"content_index\":0,\"delta\":\"bad\",\"logprobs\":[]}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("stream translation error"));
    assert!(text.contains("item_id msg_wrong"));
    assert!(text.contains("expected item_id msg_expected"));
    assert!(!text.contains("\"content\":\"bad\""));
}

#[tokio::test]
async fn rejects_output_item_done_with_mismatched_item_id() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_done_mismatch\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_expected\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n",
        "event: response.output_item.done\n",
        "data: {\"type\":\"response.output_item.done\",\"sequence_number\":3,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_wrong\",\"role\":\"assistant\",\"status\":\"completed\",\"content\":[]}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("stream translation error"));
    assert!(text.contains("item_id msg_wrong"));
    assert!(text.contains("expected item_id msg_expected"));
}

#[tokio::test]
async fn rejects_unknown_responses_events_as_protocol_drift() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_unknown\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.future_progress\n",
        "data: {\"type\":\"response.future_progress\",\"sequence_number\":2,\"detail\":\"new upstream telemetry\"}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":3,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":4,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"ok\",\"logprobs\":[]}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":5,\"response\":{\"id\":\"resp_unknown\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"completed\"}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("stream translation error"));
    assert!(text.contains("response.future_progress"));
    assert!(!text.contains("\"content\":\"ok\""));
    assert!(!text.contains("data: [DONE]"));
}

#[tokio::test]
async fn reports_unexpected_eof_before_responses_terminal_event() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_eof\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":3,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"partial\",\"logprobs\":[]}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("\"content\":\"partial\""));
    assert!(text.contains("stream translation error"));
    assert!(text.contains("Responses stream reached EOF before a terminal response event"));
    assert!(!text.contains("data: [DONE]"));
}

#[tokio::test]
async fn skips_responses_audio_events_and_continues_chat_stream_translation() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_audio\",\"object\":\"response\",\"created_at\":0,\"model\":\"gpt-audio\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_audio\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n",
        "event: response.audio.delta\n",
        "data: {\"type\":\"response.audio.delta\",\"sequence_number\":3,\"delta\":\"base64-audio\"}\n\n",
        "event: response.audio.done\n",
        "data: {\"type\":\"response.audio.done\",\"sequence_number\":4}\n\n",
        "event: response.audio.transcript.delta\n",
        "data: {\"type\":\"response.audio.transcript.delta\",\"sequence_number\":5,\"delta\":\"hidden transcript\"}\n\n",
        "event: response.audio.transcript.done\n",
        "data: {\"type\":\"response.audio.transcript.done\",\"sequence_number\":6}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":7,\"item_id\":\"msg_audio\",\"output_index\":0,\"content_index\":0,\"delta\":\"visible text\",\"logprobs\":[]}\n\n",
        "event: response.output_text.done\n",
        "data: {\"type\":\"response.output_text.done\",\"sequence_number\":8,\"item_id\":\"msg_audio\",\"output_index\":0,\"content_index\":0,\"text\":\"visible text\",\"logprobs\":[]}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":9,\"response\":{\"id\":\"resp_audio\",\"object\":\"response\",\"created_at\":0,\"model\":\"gpt-audio\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"completed\"}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("\"content\":\"visible text\""));
    assert!(text.contains("\"finish_reason\":\"stop\""));
    assert!(text.contains("data: [DONE]"));
    assert!(!text.contains("base64-audio"));
    assert!(!text.contains("hidden transcript"));
    assert!(!text.contains("stream translation error"));
}

#[tokio::test]
async fn propagates_response_error_as_stream_error() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_error\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: error\n",
        "data: {\"type\":\"error\",\"sequence_number\":2,\"code\":null,\"param\":null,\"message\":\"upstream failed\"}\n\n",
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("event: error"));
    assert!(text.contains("Responses stream error"), "{text}");
    assert!(text.contains("upstream failed"));
    assert!(!text.contains("\"finish_reason\":\"stop\""));
    assert!(!text.contains("data: [DONE]"));
}

#[tokio::test]
async fn forwards_text_suffix_from_output_text_done_snapshot() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_text_suffix\",\"object\":\"response\",\"created_at\":0,\"model\":\"test\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":3,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"Hel\",\"logprobs\":[]}\n\n",
        "event: response.output_text.done\n",
        "data: {\"type\":\"response.output_text.done\",\"sequence_number\":4,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"text\":\"Hello\",\"logprobs\":[]}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":5,\"response\":{\"id\":\"resp_text_suffix\",\"object\":\"response\",\"created_at\":0,\"model\":\"test\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"completed\"}}\n\n"
    );
    let chunks = chat_stream_payloads(&translate_responses_stream_body(body).await);
    let content = chunks
        .iter()
        .filter_map(|chunk| chunk["choices"][0]["delta"]["content"].as_str())
        .collect::<Vec<_>>();

    assert_eq!(content, ["Hel", "lo"]);
}

#[tokio::test]
async fn forwards_reasoning_suffix_from_done_snapshot() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_reasoning_suffix\",\"object\":\"response\",\"created_at\":0,\"model\":\"test\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_1\",\"summary\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.reasoning_text.delta\n",
        "data: {\"type\":\"response.reasoning_text.delta\",\"sequence_number\":3,\"item_id\":\"rs_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"plan\"}\n\n",
        "event: response.reasoning_text.done\n",
        "data: {\"type\":\"response.reasoning_text.done\",\"sequence_number\":4,\"item_id\":\"rs_1\",\"output_index\":0,\"content_index\":0,\"text\":\"planning\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":5,\"response\":{\"id\":\"resp_reasoning_suffix\",\"object\":\"response\",\"created_at\":0,\"model\":\"test\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"completed\"}}\n\n"
    );
    let chunks = chat_stream_payloads(&translate_responses_stream_body(body).await);
    let reasoning = chunks
        .iter()
        .filter_map(|chunk| chunk["choices"][0]["delta"]["reasoning_content"].as_str())
        .collect::<Vec<_>>();

    assert_eq!(reasoning, ["plan", "ning"]);
}

#[tokio::test]
async fn forwards_initial_function_call_arguments() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_initial_arguments\",\"object\":\"response\",\"created_at\":0,\"model\":\"test\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"lookup\",\"arguments\":\"{\\\"city\\\":\\\"SF\\\"}\",\"status\":\"in_progress\"}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":3,\"response\":{\"id\":\"resp_initial_arguments\",\"object\":\"response\",\"created_at\":0,\"model\":\"test\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"completed\"}}\n\n"
    );
    let chunks = chat_stream_payloads(&translate_responses_stream_body(body).await);

    assert_eq!(
        chunks[1]["choices"][0]["delta"]["tool_calls"][0]["function"]["arguments"],
        "{\"city\":\"SF\"}"
    );
}

#[tokio::test]
async fn forwards_content_present_only_in_content_part_done() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_part_done\",\"object\":\"response\",\"created_at\":0,\"model\":\"test\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n",
        "event: response.content_part.done\n",
        "data: {\"type\":\"response.content_part.done\",\"sequence_number\":3,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"fallback\",\"annotations\":[],\"logprobs\":[]}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":4,\"response\":{\"id\":\"resp_part_done\",\"object\":\"response\",\"created_at\":0,\"model\":\"test\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"completed\"}}\n\n"
    );
    let chunks = chat_stream_payloads(&translate_responses_stream_body(body).await);

    assert_eq!(chunks[1]["choices"][0]["delta"]["content"], "fallback");
}

#[tokio::test]
async fn forwards_function_argument_suffix_from_done_snapshot() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_argument_suffix\",\"object\":\"response\",\"created_at\":0,\"model\":\"test\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"lookup\",\"arguments\":\"\",\"status\":\"in_progress\"}}\n\n",
        "event: response.function_call_arguments.delta\n",
        "data: {\"type\":\"response.function_call_arguments.delta\",\"sequence_number\":3,\"item_id\":\"fc_1\",\"output_index\":0,\"delta\":\"{\\\"city\\\":\"}\n\n",
        "event: response.function_call_arguments.done\n",
        "data: {\"type\":\"response.function_call_arguments.done\",\"sequence_number\":4,\"item_id\":\"fc_1\",\"output_index\":0,\"name\":\"lookup\",\"arguments\":\"{\\\"city\\\":\\\"SF\\\"}\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":5,\"response\":{\"id\":\"resp_argument_suffix\",\"object\":\"response\",\"created_at\":0,\"model\":\"test\",\"output\":[],\"parallel_tool_calls\":false,\"tool_choice\":\"auto\",\"tools\":[],\"status\":\"completed\"}}\n\n"
    );
    let chunks = chat_stream_payloads(&translate_responses_stream_body(body).await);
    let arguments = chunks
        .iter()
        .filter_map(|chunk| {
            chunk["choices"][0]["delta"]["tool_calls"][0]["function"]["arguments"].as_str()
        })
        .collect::<Vec<_>>();

    assert_eq!(arguments, ["", "{\"city\":", "\"SF\"}"]);
}
