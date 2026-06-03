use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs as stdfs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths;
use crate::request::RequestId;
use crate::upstream::UpstreamStreamMetrics;

use super::sse::{is_terminal_event, is_tool_argument_done};
use super::summary::ResponseSummary;
use super::ResponsesUpstreamStreamSnapshot;
use crate::protocol::openai_responses::{Billing, Conversation};
use crate::protocol::ErrorObject;
use crate::sse::SseEventScanner;

pub(super) struct ResponsesStreamDiagnostics {
    request_id: RequestId,
    recent_tail: Vec<u8>,
}

impl ResponsesStreamDiagnostics {
    pub(super) fn new(request_id: RequestId) -> Self {
        Self {
            request_id,
            recent_tail: Vec::new(),
        }
    }

    pub(super) fn observe_chunk(&mut self, chunk: &[u8]) {
        const MAX_STREAM_DIAGNOSTIC_TAIL_BYTES: usize = 16 * 1024;

        self.recent_tail.extend_from_slice(chunk);
        if self.recent_tail.len() > MAX_STREAM_DIAGNOSTIC_TAIL_BYTES {
            let overflow = self.recent_tail.len() - MAX_STREAM_DIAGNOSTIC_TAIL_BYTES;
            self.recent_tail.drain(..overflow);
        }
    }

    pub(super) fn write_unfinished_tool_diagnostic(
        &self,
        snapshot: &ResponsesUpstreamStreamSnapshot,
    ) -> Option<String> {
        let logs_dir = paths::ensure_app_paths().ok()?.logs_dir;
        stdfs::create_dir_all(&logs_dir).ok()?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_millis();
        let raw_request_id: u64 = self.request_id.into();
        let path = logs_dir.join(format!(
            "unfinished-tool-diagnostic-{timestamp}-{raw_request_id:06}.json"
        ));

        let body =
            UnfinishedToolDiagnosticReport::new(self.request_id, snapshot, &self.recent_tail);

        stdfs::write(&path, serde_json::to_vec_pretty(&body).ok()?).ok()?;
        Some(path.display().to_string())
    }
}

#[derive(Serialize)]
struct UnfinishedToolDiagnosticReport {
    request_id: RequestId,
    kind: &'static str,
    error: DiagnosticErrorSection,
    upstream: DiagnosticUpstreamSection,
    stream: DiagnosticStreamSection,
    tool_arguments: ToolArgumentsSection,
    response: DiagnosticResponseSection,
    observed_summary: ResponseSummary,
    observed_error: Option<ErrorObject>,
}

impl UnfinishedToolDiagnosticReport {
    fn new(
        request_id: RequestId,
        snapshot: &ResponsesUpstreamStreamSnapshot,
        recent_tail: &[u8],
    ) -> Self {
        let tool_arguments = analyze_unfinished_tool_tail(recent_tail);
        let recent_tail_text = String::from_utf8_lossy(recent_tail).to_string();

        Self {
            request_id,
            kind: "unfinished-tool",
            error: DiagnosticErrorSection {
                message: "upstream stream ended with unfinished tool arguments",
                sequence_number: snapshot.state.sequence_number,
            },
            upstream: DiagnosticUpstreamSection::from(snapshot),
            stream: DiagnosticStreamSection::new(snapshot.metrics, recent_tail, recent_tail_text),
            tool_arguments,
            response: DiagnosticResponseSection::new(snapshot),
            observed_summary: snapshot.state.fallback_summary(),
            observed_error: snapshot.state.observed_error().cloned(),
        }
    }
}

#[derive(Serialize)]
struct DiagnosticErrorSection {
    message: &'static str,
    sequence_number: Option<u64>,
}

#[derive(Serialize)]
struct DiagnosticUpstreamSection {
    status: u16,
    content_type: String,
    transfer_encoding: String,
    sse: bool,
    ttfb_ms: u64,
}

impl From<&ResponsesUpstreamStreamSnapshot> for DiagnosticUpstreamSection {
    fn from(snapshot: &ResponsesUpstreamStreamSnapshot) -> Self {
        Self {
            status: snapshot.head.status.as_u16(),
            content_type: snapshot.head.content_type_text(),
            transfer_encoding: snapshot.head.transfer_encoding_text(),
            sse: snapshot.head.is_sse(),
            ttfb_ms: snapshot.head.ttfb.as_millis() as u64,
        }
    }
}

#[derive(Serialize)]
struct DiagnosticStreamSection {
    duration_ms: u64,
    chunks: u64,
    bytes: u64,
    avg_chunk_bytes: u64,
    recent_tail_bytes: usize,
    recent_tail_utf8_lossy: String,
}

impl DiagnosticStreamSection {
    fn new(
        metrics: UpstreamStreamMetrics,
        recent_tail: &[u8],
        recent_tail_utf8_lossy: String,
    ) -> Self {
        Self {
            duration_ms: metrics.duration_ms(),
            chunks: metrics.chunks,
            bytes: metrics.bytes,
            avg_chunk_bytes: metrics.avg_chunk_bytes(),
            recent_tail_bytes: recent_tail.len(),
            recent_tail_utf8_lossy,
        }
    }
}

#[derive(Serialize)]
struct DiagnosticResponseSection {
    background: Option<bool>,
    billing: Option<Billing>,
    conversation: Option<Conversation>,
    id: String,
    model: String,
    status: String,
    service_tier: Option<String>,
    max_output_tokens: Option<u32>,
    metadata_present: Option<bool>,
    object: String,
    parallel_tool_calls: Option<bool>,
    prompt_cache_key_present: Option<bool>,
    prompt_cache_retention: Option<String>,
    safety_identifier_present: Option<bool>,
    temperature: Option<f32>,
    top_logprobs: Option<u8>,
    top_p: Option<f32>,
    truncation: Option<String>,
    sequence_number: Option<u64>,
    snapshot_kind: Option<String>,
    #[serde(flatten)]
    summary: ResponseSummary,
    error: Option<ErrorObject>,
}

impl DiagnosticResponseSection {
    fn new(snapshot: &ResponsesUpstreamStreamSnapshot) -> Self {
        let response_snapshot = snapshot
            .state
            .latest_snapshot
            .as_ref()
            .map(|snapshot| &snapshot.projection);
        let response_summary = snapshot.state.effective_summary();

        Self {
            background: response_snapshot.and_then(|value| value.background),
            billing: response_snapshot.and_then(|value| value.billing.clone()),
            conversation: response_snapshot.and_then(|value| value.conversation.clone()),
            id: response_snapshot
                .map(|value| value.id.clone())
                .unwrap_or_default(),
            model: response_snapshot
                .map(|value| value.model.clone())
                .unwrap_or_default(),
            status: response_snapshot
                .map(|value| value.status.to_string())
                .unwrap_or_default(),
            service_tier: response_snapshot
                .and_then(|value| value.service_tier)
                .map(|value| value.to_string()),
            max_output_tokens: response_snapshot.and_then(|value| value.max_output_tokens),
            metadata_present: response_snapshot
                .and_then(|value| value.metadata.as_ref())
                .map(|value| !value.is_empty()),
            object: response_snapshot
                .map(|value| value.object.clone())
                .unwrap_or_default(),
            parallel_tool_calls: response_snapshot.and_then(|value| value.parallel_tool_calls),
            prompt_cache_key_present: response_snapshot
                .and_then(|value| value.prompt_cache_key.as_ref())
                .map(|value| !value.is_empty()),
            prompt_cache_retention: response_snapshot
                .and_then(|value| value.prompt_cache_retention)
                .map(|value| value.to_string()),
            safety_identifier_present: response_snapshot
                .and_then(|value| value.safety_identifier.as_ref())
                .map(|value| !value.is_empty()),
            temperature: response_snapshot.and_then(|value| value.temperature),
            top_logprobs: response_snapshot.and_then(|value| value.top_logprobs),
            top_p: response_snapshot.and_then(|value| value.top_p),
            truncation: response_snapshot
                .and_then(|value| value.truncation)
                .map(|value| value.to_string()),
            sequence_number: snapshot.state.sequence_number,
            snapshot_kind: snapshot
                .state
                .latest_snapshot
                .as_ref()
                .map(|value| format!("{:?}", value.kind)),
            summary: response_summary,
            error: snapshot.state.effective_error().cloned(),
        }
    }
}

#[derive(Debug, Default, Serialize)]
struct ToolArgumentsSection {
    item_id: Option<String>,
    assembled: String,
    parsed: ToolArgumentsParseResult,
    saw_arguments_done: bool,
    saw_terminal_event: bool,
    last_sequence_number: Option<u64>,
}

fn analyze_unfinished_tool_tail(recent_tail: &[u8]) -> ToolArgumentsSection {
    let mut scanner = SseEventScanner::default();
    let mut result = ToolArgumentsSection::default();

    for event in scanner.scan(recent_tail) {
        if is_terminal_event(&event) {
            result.saw_terminal_event = true;
        }
        if is_tool_argument_done(&event) {
            result.saw_arguments_done = true;
            result.saw_terminal_event = true;
        }

        let Ok(delta_event) =
            serde_json::from_str::<FunctionCallArgumentsDeltaEventData>(&event.data)
        else {
            continue;
        };
        if delta_event.kind.as_deref() != Some("response.function_call_arguments.delta") {
            continue;
        }

        if result.item_id.is_none() {
            result.item_id = delta_event.item_id;
        }
        result
            .assembled
            .push_str(delta_event.delta.as_deref().unwrap_or_default());
        result.last_sequence_number = delta_event.sequence_number;
    }

    if result.assembled.is_empty() {
        result.parsed = ToolArgumentsParseResult::Empty;
        return result;
    }

    result.parsed = match serde_json::from_str::<Value>(&result.assembled) {
        Ok(value) => ToolArgumentsParseResult::Json { value },
        Err(_) => ToolArgumentsParseResult::Incomplete {
            raw: result.assembled.clone(),
        },
    };

    result
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ToolArgumentsParseResult {
    #[default]
    Empty,
    Json {
        value: Value,
    },
    Incomplete {
        raw: String,
    },
}

#[derive(Deserialize)]
struct FunctionCallArgumentsDeltaEventData {
    #[serde(rename = "type")]
    kind: Option<String>,
    sequence_number: Option<u64>,
    item_id: Option<String>,
    delta: Option<String>,
}

#[cfg(test)]
#[path = "diagnostic_tests.rs"]
mod tests;
