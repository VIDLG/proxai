use std::collections::BTreeMap;

use serde_json::json;

use super::super::record::ValuableJson;
use super::ChatResponseFields;

fn count_map(entries: &[(&str, u64)]) -> BTreeMap<String, u64> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), *value))
        .collect()
}

#[test]
fn chat_response_fields_json_keeps_structured_calls_by_source() {
    let fields = ChatResponseFields {
        id: "chatcmpl_123".to_string(),
        model: "gpt-test".to_string(),
        service_tier: "default".to_string(),
        tok: 30,
        input: 10,
        cache: Some(2),
        output: 20,
        reasoning: 3,
        output_items: count_map(&[("message", 1), ("tool_call", 2)]),
        output_items_human: "m:1 tool_call:2".to_string(),
        finish_reasons: count_map(&[("tool_calls", 1)]),
        calls: count_map(&[("edit_file", 1), ("write_file", 1)]),
        calls_by_source: BTreeMap::from([
            ("tool".to_string(), count_map(&[("edit_file", 1)])),
            ("custom_tool".to_string(), count_map(&[("write_file", 1)])),
        ]),
        calls_human: "e:1 w:1".to_string(),
    };

    let value = fields.to_json_value();

    assert_eq!(value["calls"], json!({"edit_file": 1, "write_file": 1}));
    assert_eq!(
        value["calls_by_source"],
        json!({
            "tool": {"edit_file": 1},
            "custom_tool": {"write_file": 1}
        })
    );
    assert_eq!(value["finish_reasons"], json!({"tool_calls": 1}));
    assert!(value.get("output_items_human").is_none());
    assert!(value.get("calls_human").is_none());
}
