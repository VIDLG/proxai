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
            url: Some("https://example.test".to_string()).into(),
        },))
        .unwrap(),
        json!({"type": "open_page", "url": "https://example.test"})
    );
}

#[test]
fn serializes_output_item_kind_as_wire_discriminator() {
    assert_eq!(
        serde_json::to_value(OutputItemKind::FunctionCall).unwrap(),
        json!("function_call")
    );
}

#[test]
fn deserializes_recently_added_responses_output_item_variants() {
    let local_shell_output = json!({
        "type": "local_shell_call_output",
        "id": "shell_output_1",
        "output": "done",
        "status": "completed"
    });
    let parsed = serde_json::from_value::<OutputItem>(local_shell_output.clone()).unwrap();
    assert!(matches!(parsed, OutputItem::LocalShellCallOutput(_)));
    assert_eq!(serde_json::to_value(parsed).unwrap(), local_shell_output);

    let approval_response = json!({
        "type": "mcp_approval_response",
        "id": "approval_response_1",
        "approval_request_id": "approval_request_1",
        "approve": true,
        "reason": "approved"
    });
    let parsed = serde_json::from_value::<OutputItem>(approval_response.clone()).unwrap();
    assert!(matches!(parsed, OutputItem::McpApprovalResponse(_)));
    assert_eq!(serde_json::to_value(parsed).unwrap(), approval_response);
}

#[test]
fn deserializes_official_responses_audio_stream_events() {
    let cases = [
        (
            json!({
                "type": "response.audio.delta",
                "sequence_number": 1,
                "delta": "base64-audio"
            }),
            "response.audio.delta",
        ),
        (
            json!({
                "type": "response.audio.done",
                "sequence_number": 2
            }),
            "response.audio.done",
        ),
        (
            json!({
                "type": "response.audio.transcript.delta",
                "delta": "transcript",
                "sequence_number": 3
            }),
            "response.audio.transcript.delta",
        ),
        (
            json!({
                "type": "response.audio.transcript.done",
                "sequence_number": 4
            }),
            "response.audio.transcript.done",
        ),
    ];

    for (payload, event_type) in cases {
        let parsed = serde_json::from_value::<ResponseStreamEvent>(payload.clone()).unwrap();
        assert_eq!(parsed.as_ref(), event_type);
        assert_eq!(serde_json::to_value(parsed).unwrap(), payload);
    }
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
            description: None.into(),
            parameters: None.into(),
            strict: None.into(),
            defer_loading: None,
        }))
        .unwrap(),
        json!({
            "type": "function",
            "name": "lookup"
        })
    );
}
