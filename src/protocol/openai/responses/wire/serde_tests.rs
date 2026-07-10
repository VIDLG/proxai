use serde_json::json;

use super::*;

#[test]
fn serializes_responses_reasoning_parts_as_type_tagged_wire() {
    let reasoning = ReasoningItemContent::ReasoningText(ReasoningTextContent {
        text: "thinking".to_string(),
    });
    let summary = SummaryPart::SummaryText(SummaryTextContent {
        text: "summary".to_string(),
    });

    assert_eq!(
        serde_json::to_value(reasoning).unwrap(),
        json!({"type": "reasoning_text", "text": "thinking"})
    );
    assert_eq!(
        serde_json::to_value(summary).unwrap(),
        json!({"type": "summary_text", "text": "summary"})
    );
}

#[test]
fn serializes_responses_nested_output_unions_as_type_tagged_wire() {
    assert_eq!(
        serde_json::to_value(Annotation::FilePath(FilePath {
            file_id: "file_1".to_string(),
            index: 0,
        }))
        .unwrap(),
        json!({"type": "file_path", "file_id": "file_1", "index": 0})
    );

    assert_eq!(
        serde_json::to_value(ApplyPatchOperation::DeleteFile(
            ApplyPatchDeleteFileOperation {
                path: "src/lib.rs".to_string(),
            },
        ))
        .unwrap(),
        json!({"type": "delete_file", "path": "src/lib.rs"})
    );

    assert_eq!(
        serde_json::to_value(CodeInterpreterToolCallOutput::Logs(
            CodeInterpreterOutputLogs {
                logs: "done".to_string(),
            },
        ))
        .unwrap(),
        json!({"type": "logs", "logs": "done"})
    );

    assert_eq!(
        serde_json::to_value(ComputerAction::Screenshot).unwrap(),
        json!({"type": "screenshot"})
    );

    assert_eq!(
        serde_json::to_value(WebSearchToolCallAction::OpenPage(WebSearchActionOpenPage {
            url: Some("https://example.test".to_string()),
        },))
        .unwrap(),
        json!({"type": "open_page", "url": "https://example.test"})
    );
}

#[test]
fn rejects_unknown_responses_stream_event_types() {
    let payload = json!({
        "type": "response.future_progress",
        "sequence_number": 2,
        "detail": "new upstream telemetry"
    });

    assert!(serde_json::from_value::<ResponseStreamEvent>(payload).is_err());
}

#[test]
fn serializes_responses_shell_and_namespace_unions_as_type_tagged_wire() {
    assert_eq!(
        serde_json::to_value(ContainerNetworkPolicy::Disabled).unwrap(),
        json!({"type": "disabled"})
    );

    assert_eq!(
        serde_json::to_value(FunctionShellCallOutputOutcomeParam::Exit(
            FunctionShellCallOutputExitOutcomeParam { exit_code: 0 },
        ))
        .unwrap(),
        json!({"type": "exit", "exit_code": 0})
    );

    assert_eq!(
        serde_json::to_value(FunctionShellCallEnvironment::Local).unwrap(),
        json!({"type": "local"})
    );

    assert_eq!(
        serde_json::to_value(NamespaceToolParamTool::Function(FunctionToolParam {
            name: "lookup".to_string(),
            description: None,
            parameters: None,
            strict: None,
            defer_loading: None,
        }))
        .unwrap(),
        json!({
            "type": "function",
            "name": "lookup"
        })
    );
}
