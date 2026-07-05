use serde_json::json;

use super::*;

#[test]
fn merge_adjacent_tool_messages_groups_parallel_tool_uses_and_results() {
    let messages = vec![
        content_block_message(
            anthropic::MessageParamRole::Assistant,
            anthropic::ContentBlockParam::ToolUse(tool_use_block_param(
                "call_1",
                "lookup",
                json!({"id": 1}),
            )),
        ),
        content_block_message(
            anthropic::MessageParamRole::Assistant,
            anthropic::ContentBlockParam::ToolUse(tool_use_block_param(
                "call_2",
                "search",
                json!({"q": "proxai"}),
            )),
        ),
        content_block_message(
            anthropic::MessageParamRole::User,
            anthropic::ContentBlockParam::ToolResult(tool_result_block("call_1", "lookup result")),
        ),
        content_block_message(
            anthropic::MessageParamRole::User,
            anthropic::ContentBlockParam::ToolResult(tool_result_block("call_2", "search result")),
        ),
    ];

    let merged = merge_adjacent_tool_messages(messages);

    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].role, anthropic::MessageParamRole::Assistant);
    assert_eq!(merged[1].role, anthropic::MessageParamRole::User);
    assert_eq!(content_blocks(&merged[0]).len(), 2);
    assert_eq!(content_blocks(&merged[1]).len(), 2);
}

#[test]
fn merge_adjacent_tool_messages_does_not_absorb_regular_turns() {
    let messages = vec![
        content_block_message(
            anthropic::MessageParamRole::User,
            anthropic::ContentBlockParam::ToolResult(tool_result_block("call_1", "result")),
        ),
        user_message(anthropic::MessageParamContent::Text("continue".to_string())),
        assistant_message(anthropic::MessageParamContent::Text("ok".to_string())),
        content_block_message(
            anthropic::MessageParamRole::Assistant,
            anthropic::ContentBlockParam::ToolUse(tool_use_block_param(
                "call_2",
                "lookup",
                json!({}),
            )),
        ),
    ];

    let merged = merge_adjacent_tool_messages(messages);

    assert_eq!(merged.len(), 4);
    assert_eq!(merged[0].role, anthropic::MessageParamRole::User);
    assert_eq!(
        merged[1].content,
        anthropic::MessageParamContent::Text("continue".to_string())
    );
    assert_eq!(
        merged[2].content,
        anthropic::MessageParamContent::Text("ok".to_string())
    );
    assert_eq!(content_blocks(&merged[3]).len(), 1);
}

fn tool_result_block(id: &str, text: &str) -> anthropic::ToolResultBlockParam {
    anthropic::ToolResultBlockParam {
        tool_use_id: id.to_string(),
        content: Some(anthropic::ToolResultContentParam::Text(text.to_string())),
        is_error: Some(false),
        cache_control: None,
    }
}

fn content_blocks(message: &anthropic::MessageParam) -> &[anthropic::ContentBlockParam] {
    match &message.content {
        anthropic::MessageParamContent::Blocks(blocks) => blocks,
        anthropic::MessageParamContent::Text(_) => &[],
    }
}
