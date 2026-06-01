use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs as stdfs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths;
use crate::provider::UpstreamResponseError;
use crate::upstream::ContentType;

use super::sse::{is_terminal_event, is_tool_argument_done};
use super::summary::{ResponseOutputItemKind, ResponseSummary};
use super::ResponsesUpstreamStreamSnapshot;
use crate::protocol::openai_responses::ResponseProjection;
use crate::protocol::ErrorObject;
use crate::sse::SseEventScanner;

pub(super) struct ResponsesStreamDiagnostics {
    request_id: u64,
    recent_tail: Vec<u8>,
}

impl ResponsesStreamDiagnostics {
    pub(super) fn new(request_id: u64) -> Self {
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
        error: &UpstreamResponseError,
    ) -> Option<String> {
        write_unfinished_tool_diagnostic(self.request_id, snapshot, error, &self.recent_tail)
    }
}

fn write_unfinished_tool_diagnostic(
    request_id: u64,
    snapshot: &ResponsesUpstreamStreamSnapshot,
    error: &UpstreamResponseError,
    recent_tail: &[u8],
) -> Option<String> {
    let logs_dir = paths::ensure_app_paths().ok()?.logs_dir;
    stdfs::create_dir_all(&logs_dir).ok()?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    let path = logs_dir.join(format!(
        "unfinished-tool-diagnostic-{timestamp}-{request_id:06}.json"
    ));

    let response_snapshot = snapshot
        .state
        .snapshot
        .as_ref()
        .map(|value| &value.projection);
    let response_summary = snapshot.state.effective_summary();
    let observed_summary = snapshot.state.observed_summary();
    let observed_error = snapshot.state.observed_error();
    let sequence_number = match error {
        UpstreamResponseError::UnfinishedTool { sequence_number } => *sequence_number,
        _ => snapshot.state.sequence_number,
    };
    let recent_tail_text = String::from_utf8_lossy(recent_tail).to_string();
    let tool_arguments = analyze_unfinished_tool_tail(recent_tail);
    let body = UnfinishedToolDiagnosticBody::new(
        request_id,
        sequence_number,
        snapshot,
        response_snapshot,
        response_summary,
        observed_summary,
        observed_error,
        recent_tail,
        recent_tail_text,
        tool_arguments,
    );

    stdfs::write(&path, serde_json::to_vec_pretty(&body).ok()?).ok()?;
    Some(path.display().to_string())
}

#[derive(Serialize)]
struct UnfinishedToolDiagnosticBody {
    request_id: u64,
    kind: &'static str,
    error: DiagnosticError,
    upstream: DiagnosticUpstream,
    stream: DiagnosticStream,
    tool_arguments: ToolArgumentsDiagnostic,
    response: DiagnosticResponse,
    observed_summary: DiagnosticSummary,
    observed_error: Option<DiagnosticErrorObject>,
}

impl UnfinishedToolDiagnosticBody {
    #[allow(clippy::too_many_arguments)]
    fn new(
        request_id: u64,
        sequence_number: Option<u64>,
        snapshot: &ResponsesUpstreamStreamSnapshot,
        response_snapshot: Option<&ResponseProjection>,
        response_summary: ResponseSummary,
        observed_summary: ResponseSummary,
        observed_error: Option<&ErrorObject>,
        recent_tail: &[u8],
        recent_tail_text: String,
        tool_arguments: ToolArgumentsDiagnostic,
    ) -> Self {
        Self {
            request_id,
            kind: "unfinished-tool",
            error: DiagnosticError {
                message: "upstream stream ended with unfinished tool arguments",
                sequence_number,
            },
            upstream: DiagnosticUpstream::from(snapshot),
            stream: DiagnosticStream::new(snapshot, recent_tail, recent_tail_text),
            tool_arguments,
            response: DiagnosticResponse::new(
                snapshot,
                response_snapshot,
                response_summary,
                snapshot.state.effective_error(),
            ),
            observed_summary: DiagnosticSummary::from(observed_summary),
            observed_error: observed_error.map(DiagnosticErrorObject::from),
        }
    }
}

#[derive(Serialize)]
struct DiagnosticError {
    message: &'static str,
    sequence_number: Option<u64>,
}

#[derive(Serialize)]
struct DiagnosticUpstream {
    status: u16,
    content_type: String,
    transfer_encoding: String,
    sse: bool,
    ttfb_ms: u64,
}

impl From<&ResponsesUpstreamStreamSnapshot> for DiagnosticUpstream {
    fn from(snapshot: &ResponsesUpstreamStreamSnapshot) -> Self {
        Self {
            status: snapshot.head.status.as_u16(),
            content_type: snapshot
                .head
                .content_type
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            transfer_encoding: snapshot
                .head
                .transfer_encoding
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            sse: snapshot
                .head
                .content_type
                .as_ref()
                .is_some_and(ContentType::is_sse),
            ttfb_ms: snapshot.head.ttfb.as_millis() as u64,
        }
    }
}

#[derive(Serialize)]
struct DiagnosticStream {
    duration_ms: u64,
    chunks: u64,
    bytes: u64,
    avg_chunk_bytes: u64,
    recent_tail_bytes: usize,
    recent_tail_utf8_lossy: String,
}

impl DiagnosticStream {
    fn new(
        snapshot: &ResponsesUpstreamStreamSnapshot,
        recent_tail: &[u8],
        recent_tail_utf8_lossy: String,
    ) -> Self {
        Self {
            duration_ms: snapshot.metrics.duration_ms(),
            chunks: snapshot.metrics.chunks,
            bytes: snapshot.metrics.bytes,
            avg_chunk_bytes: snapshot.metrics.avg_chunk_bytes(),
            recent_tail_bytes: recent_tail.len(),
            recent_tail_utf8_lossy,
        }
    }
}

#[derive(Serialize)]
struct DiagnosticResponse {
    background: Option<bool>,
    billing: Option<DiagnosticBilling>,
    conversation: Option<DiagnosticConversation>,
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
    output_items: BTreeMap<ResponseOutputItemKind, u64>,
    function_calls: BTreeMap<String, u64>,
    mcp_calls: BTreeMap<String, u64>,
    error: Option<DiagnosticErrorObject>,
}

impl DiagnosticResponse {
    fn new(
        snapshot: &ResponsesUpstreamStreamSnapshot,
        response_snapshot: Option<&ResponseProjection>,
        response_summary: ResponseSummary,
        error: Option<&ErrorObject>,
    ) -> Self {
        Self {
            background: response_snapshot.and_then(|value| value.background),
            billing: response_snapshot
                .and_then(|value| value.billing.as_ref())
                .map(DiagnosticBilling::from),
            conversation: response_snapshot
                .and_then(|value| value.conversation.as_ref())
                .map(DiagnosticConversation::from),
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
                .snapshot
                .as_ref()
                .map(|value| format!("{:?}", value.kind)),
            output_items: response_summary.output_items,
            function_calls: response_summary.function_calls,
            mcp_calls: response_summary.mcp_calls,
            error: error.map(DiagnosticErrorObject::from),
        }
    }
}

#[derive(Serialize)]
struct DiagnosticBilling {
    payer: String,
}

impl From<&crate::protocol::openai_responses::Billing> for DiagnosticBilling {
    fn from(value: &crate::protocol::openai_responses::Billing) -> Self {
        Self {
            payer: value.payer.clone(),
        }
    }
}

#[derive(Serialize)]
struct DiagnosticConversation {
    id: String,
}

impl From<&crate::protocol::openai_responses::Conversation> for DiagnosticConversation {
    fn from(value: &crate::protocol::openai_responses::Conversation) -> Self {
        Self {
            id: value.id.clone(),
        }
    }
}

#[derive(Serialize)]
struct DiagnosticSummary {
    output_items: BTreeMap<ResponseOutputItemKind, u64>,
    function_calls: BTreeMap<String, u64>,
    mcp_calls: BTreeMap<String, u64>,
}

impl From<ResponseSummary> for DiagnosticSummary {
    fn from(value: ResponseSummary) -> Self {
        Self {
            output_items: value.output_items,
            function_calls: value.function_calls,
            mcp_calls: value.mcp_calls,
        }
    }
}

#[derive(Serialize)]
struct DiagnosticErrorObject {
    code: String,
    message: String,
}

impl From<&ErrorObject> for DiagnosticErrorObject {
    fn from(value: &ErrorObject) -> Self {
        Self {
            code: value.code.clone(),
            message: value.message.clone(),
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub(super) struct ToolArgumentsDiagnostic {
    pub(super) item_id: Option<String>,
    pub(super) assembled: String,
    pub(super) valid_json: bool,
    pub(super) parsed: Value,
    pub(super) saw_arguments_done: bool,
    pub(super) saw_terminal_event: bool,
    pub(super) last_sequence_number: Option<u64>,
}

pub(super) fn analyze_unfinished_tool_tail(recent_tail: &[u8]) -> ToolArgumentsDiagnostic {
    let mut scanner = SseEventScanner::default();
    let mut result = ToolArgumentsDiagnostic::default();

    for event in scanner.scan(recent_tail) {
        if is_terminal_event(&event) {
            result.saw_terminal_event = true;
        }
        if is_tool_argument_done(&event) {
            result.saw_arguments_done = true;
            result.saw_terminal_event = true;
        }

        let Some(payload) = event.payload_json() else {
            continue;
        };
        if payload.get("type").and_then(Value::as_str)
            != Some("response.function_call_arguments.delta")
        {
            continue;
        }

        let delta = payload
            .get("delta")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let sequence_number = payload.get("sequence_number").and_then(Value::as_u64);

        if result.item_id.is_none() {
            result.item_id = payload
                .get("item_id")
                .and_then(Value::as_str)
                .map(ToString::to_string);
        }
        result.assembled.push_str(delta);
        result.last_sequence_number = sequence_number;
    }

    if result.assembled.is_empty() {
        result.parsed = Value::Null;
        return result;
    }

    match serde_json::from_str::<Value>(&result.assembled) {
        Ok(value) => {
            result.valid_json = true;
            result.parsed = value;
        }
        Err(_) => {
            result.parsed = Value::String(result.assembled.clone());
        }
    }

    result
}

#[cfg(test)]
#[path = "diagnostic_tests.rs"]
mod tests;
