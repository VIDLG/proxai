use std::collections::BTreeMap;

use serde_json::json;

use super::super::record::ValuableJson;
use super::AnthropicResponseFields;

fn count_map(entries: &[(&str, u64)]) -> BTreeMap<String, u64> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), *value))
        .collect()
}

#[test]
fn anthropic_response_fields_json_keeps_structured_calls_by_source() {
    let fields = AnthropicResponseFields {
        id: "msg_123".to_string(),
        model: "claude-test".to_string(),
        service_tier: "standard_only".to_string(),
        stop_reason: "tool_use".to_string(),
        tok: 30,
        input: 10,
        cache_read: Some(2),
        cache_creation: Some(3),
        output: 20,
        output_items: count_map(&[("text", 1), ("tool_use", 2)]),
        output_items_human: "txt:1 tu:2".to_string(),
        stop_reasons: count_map(&[("tool_use", 1)]),
        calls: count_map(&[("edit_file", 1), ("web_search", 1)]),
        calls_by_source: BTreeMap::from([
            ("tool".to_string(), count_map(&[("edit_file", 1)])),
            ("server_tool".to_string(), count_map(&[("web_search", 1)])),
        ]),
        calls_human: "e:1 web_search:1".to_string(),
    };

    let value = fields.to_json_value();

    assert_eq!(value["calls"], json!({"edit_file": 1, "web_search": 1}));
    assert_eq!(
        value["calls_by_source"],
        json!({
            "tool": {"edit_file": 1},
            "server_tool": {"web_search": 1}
        })
    );
    assert_eq!(value["cache_read"], json!(2));
    assert_eq!(value["cache_creation"], json!(3));
    assert!(value.get("output_items_human").is_none());
    assert!(value.get("calls_human").is_none());
}
