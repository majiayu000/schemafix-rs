mod common;

use common::{expect_fixed, expect_invalid_untouched, run};
use schemafix::FixKind;
use serde_json::json;

fn prop(schema: serde_json::Value) -> serde_json::Value {
    json!({ "type": "object", "properties": { "v": schema } })
}

#[test]
fn converts_integer_string() {
    let (value, kinds) = expect_fixed(run(&prop(json!({"type": "integer"})), r#"{"v": "42"}"#));
    assert_eq!(value, json!({"v": 42}));
    assert_eq!(kinds, [FixKind::StringToNumber]);
}

#[test]
fn converts_exact_decimal_for_number() {
    let (value, _) = expect_fixed(run(&prop(json!({"type": "number"})), r#"{"v": "-1.25e2"}"#));
    assert_eq!(value, json!({"v": -125.0}));
}

#[test]
fn keeps_fractional_string_for_integer() {
    let value = expect_invalid_untouched(run(&prop(json!({"type": "integer"})), r#"{"v": "1.5"}"#));
    assert_eq!(value, json!({"v": "1.5"}));
}

#[test]
fn keeps_number_that_would_lose_precision() {
    let raw = r#"{"v": "12345678901234567890123"}"#;
    let value = expect_invalid_untouched(run(&prop(json!({"type": "number"})), raw));
    assert_eq!(value, json!({"v": "12345678901234567890123"}));
}

#[test]
fn keeps_non_numeric_string() {
    let value = expect_invalid_untouched(run(&prop(json!({"type": "integer"})), r#"{"v": "ten"}"#));
    assert_eq!(value, json!({"v": "ten"}));
}

#[test]
fn converts_boolean_string_any_case() {
    let (value, kinds) = expect_fixed(run(&prop(json!({"type": "boolean"})), r#"{"v": "TRUE"}"#));
    assert_eq!(value, json!({"v": true}));
    assert_eq!(kinds, [FixKind::StringToBoolean]);
}

#[test]
fn keeps_yes_for_boolean() {
    let value = expect_invalid_untouched(run(&prop(json!({"type": "boolean"})), r#"{"v": "yes"}"#));
    assert_eq!(value, json!({"v": "yes"}));
}

#[test]
fn converts_null_string_when_null_allowed() {
    let schema = prop(json!({"type": ["integer", "null"]}));
    let (value, kinds) = expect_fixed(run(&schema, r#"{"v": "null"}"#));
    assert_eq!(value, json!({"v": null}));
    assert_eq!(kinds, [FixKind::StringToNull]);
}

#[test]
fn keeps_null_string_when_null_not_allowed() {
    let value =
        expect_invalid_untouched(run(&prop(json!({"type": "integer"})), r#"{"v": "null"}"#));
    assert_eq!(value, json!({"v": "null"}));
}

#[test]
fn type_list_with_string_never_converts() {
    let schema = prop(json!({"type": ["string", "null"], "maxLength": 2}));
    let value = expect_invalid_untouched(run(&schema, r#"{"v": "null"}"#));
    assert_eq!(value, json!({"v": "null"}));
}
