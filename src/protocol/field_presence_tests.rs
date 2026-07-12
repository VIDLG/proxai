use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Nullable, OptionalNullable, RequiredNullable, deserialize_present};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct RequiredNullableFixture {
    value: RequiredNullable<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct OptionalFixture {
    #[serde(
        default,
        deserialize_with = "deserialize_present",
        skip_serializing_if = "Option::is_none"
    )]
    value: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct OptionalNullableFixture {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    value: OptionalNullable<String>,
}

#[test]
fn required_nullable_accepts_null_and_value() {
    assert_eq!(
        serde_json::from_value::<RequiredNullableFixture>(json!({ "value": null })).unwrap(),
        RequiredNullableFixture { value: None.into() }
    );
    assert_eq!(
        serde_json::from_value::<RequiredNullableFixture>(json!({ "value": "set" })).unwrap(),
        RequiredNullableFixture {
            value: Some("set".to_string()).into(),
        }
    );
}

#[test]
fn required_nullable_rejects_missing_field() {
    assert!(serde_json::from_value::<RequiredNullableFixture>(json!({})).is_err());
}

#[test]
fn required_nullable_serializes_none_as_null() {
    let value = serde_json::to_value(RequiredNullableFixture { value: None.into() }).unwrap();
    assert_eq!(value, json!({ "value": null }));
}

#[test]
fn optional_non_nullable_distinguishes_missing_from_null() {
    assert_eq!(
        serde_json::from_value::<OptionalFixture>(json!({})).unwrap(),
        OptionalFixture { value: None }
    );
    assert!(serde_json::from_value::<OptionalFixture>(json!({ "value": null })).is_err());
    assert_eq!(
        serde_json::from_value::<OptionalFixture>(json!({ "value": "set" })).unwrap(),
        OptionalFixture {
            value: Some("set".to_string()),
        }
    );
}

#[test]
fn optional_nullable_distinguishes_missing_null_and_value() {
    assert_eq!(
        serde_json::from_value::<OptionalNullableFixture>(json!({})).unwrap(),
        OptionalNullableFixture {
            value: OptionalNullable::Missing,
        }
    );
    assert_eq!(
        serde_json::from_value::<OptionalNullableFixture>(json!({ "value": null })).unwrap(),
        OptionalNullableFixture {
            value: OptionalNullable::Null,
        }
    );
    assert_eq!(
        serde_json::from_value::<OptionalNullableFixture>(json!({ "value": "set" })).unwrap(),
        OptionalNullableFixture {
            value: OptionalNullable::Value("set".to_string()),
        }
    );
}

#[test]
fn optional_fields_omit_missing_values() {
    assert_eq!(
        serde_json::to_value(OptionalFixture { value: None }).unwrap(),
        json!({})
    );
    assert_eq!(
        serde_json::to_value(OptionalNullableFixture {
            value: OptionalNullable::Missing,
        })
        .unwrap(),
        json!({})
    );
}

#[test]
fn nullable_accessors_make_null_collapse_explicit() {
    let null = Nullable::<String>::null();
    assert!(null.is_null());
    assert!(!null.is_non_null());
    assert_eq!(null.as_non_null(), None);
    assert_eq!(null.into_non_null(), None);

    let value = Nullable::from("set".to_string());
    assert!(!value.is_null());
    assert!(value.is_non_null());
    assert_eq!(value.as_non_null().map(String::as_str), Some("set"));
    assert_eq!(value.into_non_null().as_deref(), Some("set"));
}

#[test]
fn nullable_json_value_prefers_carrier_null_state() {
    let null: RequiredNullable<serde_json::Value> = serde_json::from_value(json!(null)).unwrap();
    assert!(null.is_null());

    let object: RequiredNullable<serde_json::Value> =
        serde_json::from_value(json!({ "key": "value" })).unwrap();
    assert_eq!(object.as_non_null(), Some(&json!({ "key": "value" })));
}

#[test]
fn optional_nullable_accessors_explicitly_collapse_missing_and_null() {
    let missing = OptionalNullable::<String>::Missing;
    let null: OptionalNullable<String> = OptionalNullable::Null;
    let value = OptionalNullable::Value("set".to_string());

    assert!(missing.is_missing());
    assert!(null.is_null());
    assert!(value.is_non_null());
    assert_eq!(missing.as_non_null(), None);
    assert_eq!(null.as_non_null(), None);
    assert_eq!(value.as_non_null().map(String::as_str), Some("set"));

    assert_eq!(missing.into_non_null(), None);
    assert_eq!(null.into_non_null(), None);
    assert_eq!(value.into_non_null().as_deref(), Some("set"));
}

#[test]
fn nullable_map_preserves_explicit_null() {
    assert_eq!(
        Nullable::<String>::null().map(|value| value.len()),
        Nullable::null()
    );
    assert_eq!(
        Nullable::from("set".to_string()).map(|value| value.len()),
        3.into()
    );
}

#[test]
fn optional_nullable_serializes_present_null() {
    let value = OptionalNullableFixture {
        value: OptionalNullable::Null,
    };
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        json!({ "value": null })
    );
}
