use serde_json::{Map, Value};

use super::insert_nulls;

pub(super) fn normalize_message_object(object: &mut Map<String, Value>) {
    normalize_message_status_fields(object);
}

pub(super) fn normalize_message_delta(delta: &mut Map<String, Value>) {
    normalize_message_status_fields(delta);
}

fn normalize_message_status_fields(object: &mut Map<String, Value>) {
    insert_nulls(
        object,
        &["container", "stop_details", "stop_reason", "stop_sequence"],
    );
    normalize_refusal_stop_details(object.get_mut("stop_details"));
}

fn normalize_refusal_stop_details(value: Option<&mut Value>) {
    let Some(stop_details) = value.and_then(Value::as_object_mut) else {
        return;
    };
    insert_nulls(stop_details, &["category", "explanation"]);
}

pub(super) fn normalize_message_usage(usage: &mut Map<String, Value>) {
    insert_nulls(
        usage,
        &[
            "cache_creation",
            "cache_creation_input_tokens",
            "cache_read_input_tokens",
            "inference_geo",
            "server_tool_use",
            "service_tier",
        ],
    );
}

pub(super) fn normalize_message_delta_usage(usage: &mut Map<String, Value>) {
    insert_nulls(
        usage,
        &[
            "cache_creation_input_tokens",
            "cache_read_input_tokens",
            "input_tokens",
            "server_tool_use",
        ],
    );
}

pub(super) fn normalize_content_block(block: &mut Map<String, Value>) {
    match block.get("type").and_then(Value::as_str) {
        Some("text") => normalize_text_block(block),
        Some("web_search_tool_result") => normalize_web_search_tool_result_block(block),
        Some("web_fetch_tool_result") => normalize_web_fetch_tool_result_block(block),
        Some("tool_search_tool_result") => normalize_tool_search_tool_result_block(block),
        Some("text_editor_code_execution_tool_result") => {
            normalize_text_editor_tool_result_block(block);
        }
        _ => {}
    }
}

fn normalize_text_block(block: &mut Map<String, Value>) {
    insert_nulls(block, &["citations"]);
    normalize_citations(block.get_mut("citations"));
}

fn normalize_citations(value: Option<&mut Value>) {
    let Some(citations) = value.and_then(Value::as_array_mut) else {
        return;
    };

    for citation in citations {
        let Some(citation) = citation.as_object_mut() else {
            continue;
        };
        match citation.get("type").and_then(Value::as_str) {
            Some("char_location") | Some("page_location") | Some("content_block_location") => {
                insert_nulls(citation, &["document_title", "file_id"]);
            }
            Some("web_search_result_location") | Some("search_result_location") => {
                insert_nulls(citation, &["title"]);
            }
            _ => {}
        }
    }
}

fn normalize_web_search_tool_result_block(block: &mut Map<String, Value>) {
    let Some(content) = block.get_mut("content") else {
        return;
    };

    if let Some(results) = content.as_array_mut() {
        for result in results {
            let Some(result) = result.as_object_mut() else {
                continue;
            };
            insert_nulls(result, &["page_age"]);
        }
    }
}

fn normalize_web_fetch_tool_result_block(block: &mut Map<String, Value>) {
    let Some(results) = block.get_mut("content").and_then(Value::as_array_mut) else {
        return;
    };

    for result in results {
        let Some(result) = result.as_object_mut() else {
            continue;
        };
        insert_nulls(result, &["retrieved_at"]);
        if let Some(document) = result.get_mut("content").and_then(Value::as_object_mut) {
            normalize_document_block(document);
        }
    }
}

fn normalize_document_block(block: &mut Map<String, Value>) {
    insert_nulls(block, &["citations", "title"]);
}

fn normalize_tool_search_tool_result_block(block: &mut Map<String, Value>) {
    let Some(content) = block.get_mut("content").and_then(Value::as_object_mut) else {
        return;
    };

    if content.contains_key("error_code") {
        insert_nulls(content, &["error_message"]);
    }
}

fn normalize_text_editor_tool_result_block(block: &mut Map<String, Value>) {
    let Some(content) = block.get_mut("content").and_then(Value::as_object_mut) else {
        return;
    };

    match content.get("type").and_then(Value::as_str) {
        Some("text_editor_code_execution_str_replace_result") => insert_nulls(
            content,
            &["lines", "new_lines", "new_start", "old_lines", "old_start"],
        ),
        Some("text_editor_code_execution_view_result") => {
            insert_nulls(content, &["num_lines", "start_line", "total_lines"]);
        }
        Some("text_editor_code_execution_tool_result_error") => {
            insert_nulls(content, &["error_message"]);
        }
        _ => {}
    }
}
