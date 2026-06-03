use tracing::Level;

use super::{base36, format_event_line, short_request_id, DurationThresholds, LogFields};

#[test]
fn short_request_id_compacts_millisecond_timestamp() {
    assert_eq!(short_request_id("1778228311188"), "wn82ac");
}

#[test]
fn short_request_id_falls_back_to_last_six_chars_for_non_numeric_ids() {
    assert_eq!(short_request_id("request-abcdef"), "abcdef");
}

#[test]
fn base36_encodes_zero_and_regular_values() {
    assert_eq!(base36(0), "0");
    assert_eq!(base36(35), "z");
    assert_eq!(base36(36), "10");
}

#[test]
fn forward_line_renders_translation_tools_and_request_id() {
    let mut fields = LogFields::default();
    fields.insert("event", "fwd");
    fields.insert("method", "POST");
    fields.insert("path", "/v1/chat/completions");
    fields.insert("request_protocol_alias", "chat");
    fields.insert("provider", "anthropic");
    fields.insert("provider_protocol_alias", "ant");
    fields.insert("provider_request_bytes", "1536");
    fields.insert("model", "claude-sonnet");
    fields.insert("stream", "true");
    fields.insert("max_output_tokens", "4096");
    fields.insert("request_hints", "tools[f:2(e,sh)] tc:auto");
    fields.insert("request_id", "1778228311188");

    let mut line = String::new();
    format_event_line(
        &mut line,
        &Level::INFO,
        &fields,
        false,
        &DurationThresholds::default(),
    )
    .unwrap();

    assert!(line.contains("fwd  POST /v1/chat/completions"));
    assert!(line.contains("claude-sonnet chat->anthropic/ant stream max=4.1K"));
    assert!(line.contains("tools[f:2(e,sh)]"));
    assert!(line.contains("tc:auto"));
    assert!(line.contains("req=wn82ac"));
    assert!(!line.contains("provider="));
    assert!(!line.contains("route="));
}

#[test]
fn forward_line_renders_request_effort_when_present() {
    let mut fields = LogFields::default();
    fields.insert("event", "fwd");
    fields.insert("method", "POST");
    fields.insert("path", "/v1/responses");
    fields.insert("request_protocol_alias", "resp");
    fields.insert("provider", "openai");
    fields.insert("provider_protocol_alias", "resp");
    fields.insert("model", "gpt-5.5");
    fields.insert("reasoning_effort", "high");

    let mut line = String::new();
    format_event_line(
        &mut line,
        &Level::INFO,
        &fields,
        false,
        &DurationThresholds::default(),
    )
    .unwrap();

    assert!(line.contains("gpt-5.5/high resp->openai/resp"));
}

#[test]
fn forward_line_omits_protocol_when_no_translation() {
    let mut fields = LogFields::default();
    fields.insert("event", "fwd");
    fields.insert("method", "POST");
    fields.insert("path", "/v1/chat/completions");
    fields.insert("request_protocol_alias", "chat");
    fields.insert("provider", "openai_default");
    fields.insert("provider_protocol_alias", "chat");
    fields.insert("model", "gpt-5.5");

    let mut line = String::new();
    format_event_line(
        &mut line,
        &Level::INFO,
        &fields,
        false,
        &DurationThresholds::default(),
    )
    .unwrap();

    assert!(line.contains("fwd  POST /v1/chat/completions"));
    assert!(line.contains("gpt-5.5 chat->openai/chat"));
    assert!(!line.contains(" openai_chat_completions"));
    assert!(!line.contains("provider="));
    assert!(!line.contains("route="));
}

#[test]
fn wait_line_renders_idle_duration_traffic_and_request_id() {
    let mut fields = LogFields::default();
    fields.insert("event", "wait");
    fields.insert("phase", "tool_args");
    fields.insert("idle_ms", "5500");
    fields.insert("duration_ms", "12345");
    fields.insert("down", "2048");
    fields.insert("chunks", "263");
    fields.insert("response_id", "...053fdded");
    fields.insert("seq", "81");
    fields.insert("response_status", "in_progress");
    fields.insert("pending_tool_items", "1");
    fields.insert("request_id", "1778228311188");

    let mut line = String::new();
    format_event_line(
        &mut line,
        &Level::INFO,
        &fields,
        false,
        &DurationThresholds::default(),
    )
    .unwrap();

    assert!(line.contains("wait  idle=5.5s tool_args dur=12.345s"));
    assert!(line.contains("↓2.0KB"));
    assert!(line.contains("chunks=263"));
    assert!(line.contains("resp=...053fdded"));
    assert!(line.contains("seq=81"));
    assert!(line.contains("state=in_progress"));
    assert!(line.contains("pending=1"));
    assert!(line.contains("req=wn82ac"));
}

#[test]
fn end_line_uses_human_summary_fields_and_omits_default_sse_ct() {
    let mut fields = LogFields::default();
    fields.insert("event", "end");
    fields.insert("status", "200");
    fields.insert("ttfb_ms", "120");
    fields.insert("duration_ms", "1250");
    fields.insert("down", "2048");
    fields.insert("ct", "text/event-stream");
    fields.insert("sse", "true");
    fields.insert("chunks", "8");
    fields.insert("avg_chunk_bytes", "256");
    fields.insert("response_id", "...12345678");
    fields.insert("input", "1000");
    fields.insert("output", "2000");
    fields.insert("tok", "3000");
    fields.insert("cache", "500");
    fields.insert("reasoning", "100");
    fields.insert("output_items_human", "m:1 fn:2");
    fields.insert("calls_human", "e:1 sh:1");

    let mut line = String::new();
    format_event_line(
        &mut line,
        &Level::INFO,
        &fields,
        false,
        &DurationThresholds::default(),
    )
    .unwrap();

    assert!(line.contains("end  200"));
    assert!(line.contains("tok[↑1.0K ↓2.0K Σ3.0K $500 ?100]"));
    assert!(line.contains("stream[8/256B]"));
    assert!(line.contains("resp=...12345678"));
    assert!(line.contains("out[m:1 fn:2]"));
    assert!(line.contains("calls[e:1 sh:1]"));
    assert!(!line.contains("text/event-stream"));
}

#[test]
fn end_line_omits_default_tier_and_empty_human_output() {
    let mut fields = LogFields::default();
    fields.insert("event", "end");
    fields.insert("status", "200");
    fields.insert("service_tier", "default");
    fields.insert("output_items_human", "");

    let mut line = String::new();
    format_event_line(
        &mut line,
        &Level::INFO,
        &fields,
        false,
        &DurationThresholds::default(),
    )
    .unwrap();

    assert!(!line.contains("tier=default"));
    assert!(!line.contains(" out["));
}

#[test]
fn end_line_keeps_non_default_tier_and_complex_output() {
    let mut fields = LogFields::default();
    fields.insert("event", "end");
    fields.insert("status", "200");
    fields.insert("service_tier", "flex");
    fields.insert("output_items_human", "m:1 fn:1");

    let mut line = String::new();
    format_event_line(
        &mut line,
        &Level::INFO,
        &fields,
        false,
        &DurationThresholds::default(),
    )
    .unwrap();

    assert!(line.contains("tier=flex"));
    assert!(line.contains("out[m:1 fn:1]"));
}

#[test]
fn end_line_renders_response_effort_only() {
    let mut fields = LogFields::default();
    fields.insert("event", "end");
    fields.insert("status", "200");
    fields.insert("model", "gpt-5.5");
    fields.insert("request_reasoning_effort", "high");
    fields.insert("reasoning_effort", "medium");

    let mut line = String::new();
    format_event_line(
        &mut line,
        &Level::INFO,
        &fields,
        false,
        &DurationThresholds::default(),
    )
    .unwrap();

    assert!(line.contains("gpt-5.5/medium"));
    assert!(!line.contains("high->medium"));
}

#[test]
fn end_line_renders_anthropic_cache_tokens() {
    let mut fields = LogFields::default();
    fields.insert("event", "end");
    fields.insert("status", "200");
    fields.insert("input", "100");
    fields.insert("output", "50");
    fields.insert("cache_read", "25");
    fields.insert("cache_creation", "10");

    let mut line = String::new();
    format_event_line(
        &mut line,
        &Level::INFO,
        &fields,
        false,
        &DurationThresholds::default(),
    )
    .unwrap();

    assert!(line.contains("tok[↑100 ↓50 $r25 $w10]"));
}
