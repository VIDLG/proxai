use serde_json::json;

use crate::protocol::OptionalNullable;

use super::{AdditionalToolsItemRole, Item};

#[test]
fn additional_tools_input_item_uses_the_official_input_shape() {
    let omitted_id = serde_json::from_value::<Item>(json!({
        "type": "additional_tools",
        "role": "developer",
        "tools": []
    }))
    .unwrap();
    let Item::AdditionalTools(item) = omitted_id else {
        panic!("expected additional_tools input item");
    };
    assert!(item.id.is_missing());
    assert_eq!(item.role, AdditionalToolsItemRole::Developer);

    let null_id = serde_json::from_value::<Item>(json!({
        "type": "additional_tools",
        "id": null,
        "role": "developer",
        "tools": []
    }))
    .unwrap();
    let Item::AdditionalTools(item) = null_id else {
        panic!("expected additional_tools input item");
    };
    assert_eq!(item.id, OptionalNullable::Null);

    assert!(
        serde_json::from_value::<Item>(json!({
            "type": "additional_tools",
            "id": "at_123",
            "role": "assistant",
            "tools": []
        }))
        .is_err()
    );
}
