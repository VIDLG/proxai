use std::collections::BTreeMap;

use serde_json::json;

use super::super::record::ValuableJson;
use super::ResponseFields;

fn count_map(entries: &[(&str, u64)]) -> BTreeMap<String, u64> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), *value))
        .collect()
}

#[test]
fn response_fields_json_keeps_structured_calls_by_source() {
    let fields = ResponseFields {
        id: "resp_123".to_string(),
        model: "gpt-test".to_string(),
        reasoning_effort: "medium".to_string(),
        status: "completed".to_string(),
        service_tier: "default".to_string(),
        incomplete_reason: String::new(),
        error_code: String::new(),
        error_message: String::new(),
        error_param: String::new(),
        sequence_number: Some(7),
        tok: 30,
        input: 10,
        cache: Some(2),
        output: 20,
        reasoning: 3,
        output_items: count_map(&[("message", 1), ("function_call", 2)]),
        output_items_human: "m:1 fn:2".to_string(),
        calls: count_map(&[("edit_file", 1), ("terminal", 1)]),
        calls_by_source: BTreeMap::from([
            ("function".to_string(), count_map(&[("edit_file", 1)])),
            ("mcp".to_string(), count_map(&[("terminal", 1)])),
        ]),
        calls_human: "e:1 sh:1".to_string(),
    };

    let value = fields.to_json_value();

    assert_eq!(
        value["output_items"],
        json!({"function_call": 2, "message": 1})
    );
    assert_eq!(value["calls"], json!({"edit_file": 1, "terminal": 1}));
    assert_eq!(
        value["calls_by_source"],
        json!({
            "function": {"edit_file": 1},
            "mcp": {"terminal": 1}
        })
    );
    assert!(value.get("output_items_human").is_none());
    assert!(value.get("calls_human").is_none());
}
