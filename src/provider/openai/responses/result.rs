use crate::logging;
use crate::provider::UpstreamResponseError;

use super::diagnostic::ResponsesStreamDiagnostics;
use super::{ResponsesUpstreamEvent, ResponsesUpstreamStreamSnapshot};

#[derive(Debug, Clone)]
pub(super) enum ResponsesStreamResult {
    StreamError { error: UpstreamResponseError },
    Completed,
    UnfinishedTool { error: UpstreamResponseError },
    Closed,
}

pub(super) fn emit_responses_stream_result(
    span: &tracing::Span,
    diagnostics: &ResponsesStreamDiagnostics,
    snapshot: Box<ResponsesUpstreamStreamSnapshot>,
    result: ResponsesStreamResult,
) {
    match result {
        ResponsesStreamResult::StreamError { error } => {
            span.in_scope(|| {
                logging::ResponsesLogRecord::from_event(&ResponsesUpstreamEvent::Error {
                    error,
                    snapshot,
                })
                .emit()
            });
        }
        ResponsesStreamResult::Completed => {
            span.in_scope(|| {
                logging::ResponsesLogRecord::from_event(&ResponsesUpstreamEvent::Completed {
                    snapshot,
                })
                .emit()
            });
        }
        ResponsesStreamResult::UnfinishedTool { error } => {
            let diagnostic_path = diagnostics.write_unfinished_tool_diagnostic(&snapshot, &error);
            span.in_scope(|| {
                logging::emit_responses_stream_error_with_diagnostic(
                    &snapshot,
                    &error,
                    diagnostic_path.as_deref(),
                )
            });
        }
        ResponsesStreamResult::Closed => {
            span.in_scope(|| {
                logging::ResponsesLogRecord::from_event(&ResponsesUpstreamEvent::Closed {
                    snapshot,
                })
                .emit()
            });
        }
    }
}
