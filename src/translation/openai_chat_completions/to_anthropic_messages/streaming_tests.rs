use axum::body::{Body, to_bytes};
use axum::http::{Response, header};
use serde_json::json;

use crate::http_support::into_byte_stream;

use super::super::translate_streaming_stream;

/// Translate a Chat Completions SSE stream into the Anthropic Messages SSE
/// stream produced by `translate_streaming_stream`, returning the parsed
/// Anthropic event payloads (one per `data:` line, in order).
fn anthropic_message_payloads(body: &str) -> Vec<serde_json::Value> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .map(|data| serde_json::from_str(data).unwrap())
        .collect()
}

async fn run_translation(stream: String) -> String {
    let mut response = Response::new(Body::from(stream));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );

    let response =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(response), usize::MAX)
        .await
        .unwrap();
    std::str::from_utf8(&body).unwrap().to_string()
}

/// Build a single Chat Completions stream chunk with one assistant choice.
fn chat_chunk(
    id: &str,
    model: &str,
    index: u32,
    delta: serde_json::Value,
    finish_reason: Option<&str>,
) -> String {
    let chunk = json!({
        "id": id,
        "object": "chat.completion.chunk",
        "created": 0,
        "model": model,
        "choices": [{
            "index": index,
            "delta": delta,
            "finish_reason": finish_reason,
            "logprobs": null,
        }],
        "usage": null,
    });
    format!("data: {chunk}\n\n")
}

/// `[DONE]` sentinel that terminates a Chat Completions stream.
fn done_sentinel() -> String {
    "data: [DONE]\n\n".to_string()
}

const CHAT_ID: &str = "chatcmpl_test";
const CHAT_MODEL: &str = "glm-5.1";

#[tokio::test]
async fn translates_chat_text_stream_to_anthropic_messages_events() {
    let stream = concat_streams([
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({ "role": "assistant" }), None),
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({ "content": "hello" }), None),
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({ "content": " world" }), None),
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({}), Some("stop")),
        done_sentinel(),
    ]);

    let body = run_translation(stream).await;
    let events = anthropic_message_payloads(&body);

    // message_start carries the assistant envelope with id prefixed `msg_`.
    assert_eq!(events[0]["type"], "message_start");
    assert_eq!(events[0]["message"]["id"], format!("msg_{CHAT_ID}"));
    assert_eq!(events[0]["message"]["model"], CHAT_MODEL);
    assert_eq!(events[0]["message"]["role"], "assistant");

    // First non-empty content opens a text block and carries the text in the
    // content_block_start payload; subsequent deltas reuse the open block.
    assert_eq!(events[1]["type"], "content_block_start");
    assert_eq!(events[1]["index"], 0);
    assert_eq!(events[1]["content_block"]["type"], "text");
    assert_eq!(events[1]["content_block"]["text"], "hello");
    assert_eq!(events[2]["type"], "content_block_delta");
    assert_eq!(events[2]["index"], 0);
    assert_eq!(events[2]["delta"]["type"], "text_delta");
    assert_eq!(events[2]["delta"]["text"], " world");

    // finish_reason=stop closes the open text block and emits message_delta +
    // message_stop. message_delta carries the Anthropic stop_reason.
    assert_eq!(events[3]["type"], "content_block_stop");
    assert_eq!(events[3]["index"], 0);
    assert_eq!(events[4]["type"], "message_delta");
    assert_eq!(events[4]["delta"]["stop_reason"], "end_turn");
    assert_eq!(events[5]["type"], "message_stop");
}

#[tokio::test]
async fn translates_chat_tool_call_stream_into_anthropic_tool_use_blocks() {
    // Tool calls arrive across multiple chunks: the first carries id + name,
    // subsequent chunks carry argument fragments indexed by Chat's tool_calls[].index.
    let stream = concat_streams([
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({ "role": "assistant" }), None),
        chat_chunk(
            CHAT_ID,
            CHAT_MODEL,
            0,
            json!({
                "tool_calls": [{
                    "index": 0,
                    "id": "call_1",
                    "type": "function",
                    "function": { "name": "get_weather", "arguments": "" },
                }],
            }),
            None,
        ),
        chat_chunk(
            CHAT_ID,
            CHAT_MODEL,
            0,
            json!({
                "tool_calls": [{
                    "index": 0,
                    "function": { "arguments": "{\"city\":" },
                }],
            }),
            None,
        ),
        chat_chunk(
            CHAT_ID,
            CHAT_MODEL,
            0,
            json!({
                "tool_calls": [{
                    "index": 0,
                    "function": { "arguments": "\"Paris\"}" },
                }],
            }),
            None,
        ),
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({}), Some("tool_calls")),
        done_sentinel(),
    ]);

    let body = run_translation(stream).await;
    let events = anthropic_message_payloads(&body);

    // Tool-call start emits content_block_start with the tool_use id + name.
    let block_start = &events[1];
    assert_eq!(block_start["type"], "content_block_start");
    assert_eq!(block_start["index"], 0);
    assert_eq!(block_start["content_block"]["type"], "tool_use");
    assert_eq!(block_start["content_block"]["id"], "call_1");
    assert_eq!(block_start["content_block"]["name"], "get_weather");

    // Each argument fragment becomes an input_json_delta on the same block.
    assert_eq!(events[2]["type"], "content_block_delta");
    assert_eq!(events[2]["delta"]["type"], "input_json_delta");
    assert_eq!(events[2]["delta"]["partial_json"], "{\"city\":");
    assert_eq!(events[3]["delta"]["partial_json"], "\"Paris\"}");

    // finish_reason=tool_calls closes the block and ends the Anthropic turn.
    assert_eq!(events[4]["type"], "content_block_stop");
    assert_eq!(events[5]["type"], "message_delta");
    assert_eq!(events[5]["delta"]["stop_reason"], "tool_use");
    assert_eq!(events[6]["type"], "message_stop");
}

#[tokio::test]
async fn translates_chat_parallel_tool_calls_into_sequential_anthropic_blocks() {
    // Chat streams parallel tool calls by interleaving tool_calls entries with
    // different `index` values. Anthropic represents each as a separate
    // content_block with its own `index` in the assistant message.
    let stream = concat_streams([
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({ "role": "assistant" }), None),
        chat_chunk(
            CHAT_ID,
            CHAT_MODEL,
            0,
            json!({
                "tool_calls": [
                    { "index": 0, "id": "call_a", "type": "function",
                      "function": { "name": "read", "arguments": "" } },
                    { "index": 1, "id": "call_b", "type": "function",
                      "function": { "name": "read", "arguments": "" } },
                ],
            }),
            None,
        ),
        chat_chunk(
            CHAT_ID,
            CHAT_MODEL,
            0,
            json!({
                "tool_calls": [
                    { "index": 0, "function": { "arguments": "{\"p\":\"a\"}" } },
                    { "index": 1, "function": { "arguments": "{\"p\":\"b\"}" } },
                ],
            }),
            None,
        ),
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({}), Some("tool_calls")),
        done_sentinel(),
    ]);

    let body = run_translation(stream).await;
    let events = anthropic_message_payloads(&body);

    // Two content_block_start events on consecutive Anthropic block indexes.
    assert_eq!(events[1]["type"], "content_block_start");
    assert_eq!(events[1]["index"], 0);
    assert_eq!(events[1]["content_block"]["id"], "call_a");
    assert_eq!(events[2]["type"], "content_block_start");
    assert_eq!(events[2]["index"], 1);
    assert_eq!(events[2]["content_block"]["id"], "call_b");

    // Argument deltas are routed to their own block index.
    assert_eq!(events[3]["index"], 0);
    assert_eq!(events[3]["delta"]["partial_json"], "{\"p\":\"a\"}");
    assert_eq!(events[4]["index"], 1);
    assert_eq!(events[4]["delta"]["partial_json"], "{\"p\":\"b\"}");

    // Both blocks close before message_delta.
    assert_eq!(events[5]["type"], "content_block_stop");
    assert_eq!(events[5]["index"], 0);
    assert_eq!(events[6]["type"], "content_block_stop");
    assert_eq!(events[6]["index"], 1);
    assert_eq!(events[7]["type"], "message_delta");
}

#[tokio::test]
async fn translates_chat_refusal_to_anthropic_refusal_stop_reason() {
    let stream = concat_streams([
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({ "role": "assistant" }), None),
        chat_chunk(
            CHAT_ID,
            CHAT_MODEL,
            0,
            json!({ "refusal": "I can't help with that." }),
            None,
        ),
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({}), Some("stop")),
        done_sentinel(),
    ]);

    let body = run_translation(stream).await;
    let events = anthropic_message_payloads(&body);

    // Refusal text opens a text block (carrying the wording visibly) and the
    // message_delta then reports the refusal stop_reason so Anthropic
    // semantics see the refusal as a terminal state.
    assert_eq!(events[1]["type"], "content_block_start");
    assert_eq!(events[1]["content_block"]["type"], "text");
    assert_eq!(
        events[1]["content_block"]["text"],
        "I can't help with that."
    );

    let stop = &events[events.len() - 2];
    assert_eq!(stop["type"], "message_delta");
    assert_eq!(stop["delta"]["stop_reason"], "refusal");
}

#[tokio::test]
async fn attaches_usage_from_terminal_usage_only_chunk() {
    // OpenAI's stream_options.include_usage sends a trailing chunk with empty
    // choices and the cumulative usage. It must arrive after finish_reason.
    let stream = concat_streams([
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({ "role": "assistant" }), None),
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({ "content": "hi" }), None),
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({}), Some("stop")),
        usage_only_chunk(CHAT_ID, CHAT_MODEL, 7, 3),
        done_sentinel(),
    ]);

    let body = run_translation(stream).await;
    let events = anthropic_message_payloads(&body);

    // The terminal message_delta carries the usage from the trailing chunk.
    let delta = &events[events.len() - 2];
    assert_eq!(delta["type"], "message_delta");
    assert_eq!(delta["usage"]["input_tokens"], 7);
    assert_eq!(delta["usage"]["output_tokens"], 3);
}

#[tokio::test]
async fn rejects_chat_stream_without_representable_content_at_finish() {
    let stream = concat_streams([
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({ "role": "assistant" }), None),
        // No content, refusal, or tool calls before finish_reason arrives.
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({}), Some("stop")),
        done_sentinel(),
    ]);

    let body = run_translation(stream).await;
    assert!(body.contains("without Anthropic-representable content"));
}

#[tokio::test]
async fn rejects_chat_stream_mixing_content_and_refusal() {
    let stream = concat_streams([
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({ "role": "assistant" }), None),
        chat_chunk(
            CHAT_ID,
            CHAT_MODEL,
            0,
            json!({ "content": "partial answer" }),
            None,
        ),
        // Refusal after content is illegal: Anthropic refusal semantics live
        // on the message-level stop fields and cannot retract sent text.
        chat_chunk(
            CHAT_ID,
            CHAT_MODEL,
            0,
            json!({ "refusal": "actually, no" }),
            None,
        ),
        done_sentinel(),
    ]);

    let body = run_translation(stream).await;
    assert!(body.contains("both content and refusal"));
}

#[tokio::test]
async fn rejects_chat_stream_with_multiple_choices() {
    let first = json!({
        "id": CHAT_ID,
        "object": "chat.completion.chunk",
        "created": 0,
        "model": CHAT_MODEL,
        "choices": [
            { "index": 0, "delta": { "role": "assistant" }, "finish_reason": null, "logprobs": null },
            { "index": 1, "delta": { "role": "assistant" }, "finish_reason": null, "logprobs": null },
        ],
        "usage": null,
    });
    let stream = format!("data: {first}\n\n{}", done_sentinel());

    let body = run_translation(stream).await;
    assert!(body.contains("exactly one assistant message"));
}

#[tokio::test]
async fn rejects_chat_stream_changing_id_mid_stream() {
    let stream = concat_streams([
        chat_chunk(CHAT_ID, CHAT_MODEL, 0, json!({ "role": "assistant" }), None),
        chat_chunk(
            "chatcmpl_other",
            CHAT_MODEL,
            0,
            json!({ "content": "x" }),
            None,
        ),
        done_sentinel(),
    ]);

    let body = run_translation(stream).await;
    assert!(body.contains("Chat stream changed id"));
}

#[tokio::test]
async fn rejects_chat_choice_with_logprobs() {
    let chunk = json!({
        "id": CHAT_ID,
        "object": "chat.completion.chunk",
        "created": 0,
        "model": CHAT_MODEL,
        "choices": [{
            "index": 0,
            "delta": { "role": "assistant" },
            "finish_reason": null,
            "logprobs": { "content": [] },
        }],
        "usage": null,
    });
    let stream = format!("data: {chunk}\n\n{}", done_sentinel());

    let body = run_translation(stream).await;
    assert!(body.contains("logprobs cannot be represented"));
}

#[tokio::test]
async fn rejects_done_sentinel_before_any_assistant_chunk() {
    let body = run_translation(done_sentinel()).await;
    assert!(body.contains("before any assistant message chunk"));
}

fn concat_streams<I, S>(streams: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    streams
        .into_iter()
        .map(|s| s.as_ref().to_string())
        .collect()
}

fn usage_only_chunk(id: &str, model: &str, prompt_tokens: u32, completion_tokens: u32) -> String {
    let chunk = json!({
        "id": id,
        "object": "chat.completion.chunk",
        "created": 0,
        "model": model,
        "choices": [],
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens,
            "prompt_tokens_details": null,
            "completion_tokens_details": null,
        },
    });
    format!("data: {chunk}\n\n")
}
