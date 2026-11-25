mod common;

use common::{expect_fixed, expect_invalid_untouched, run};
use serde_json::json;

fn schema() -> serde_json::Value {
    json!({
        "$defs": {
            "Unit": {"enum": ["celsius", "fahrenheit"]},
            "Reading": {
                "type": "object",
                "properties": {"temp": {"type": "number"}, "unit": {"$ref": "#/$defs/Unit"}},
                "required": ["temp", "unit"]
            }
        },
        "type": "object",
        "properties": {"readings": {"type": "array", "items": {"$ref": "#/$defs/Reading"}}}
    })
}

#[test]
fn follows_local_defs() {
    let raw = r#"{"readings": [{"temp": "21.5", "unit": "Celsius"}]}"#;
    let (value, _) = expect_fixed(run(&schema(), raw));
    assert_eq!(
        value,
        json!({"readings": [{"temp": 21.5, "unit": "celsius"}]})
    );
}

#[test]
fn follows_legacy_definitions() {
    let schema = json!({
        "definitions": {"N": {"type": "integer"}},
        "type": "object",
        "properties": {"n": {"$ref": "#/definitions/N"}}
    });
    let (value, _) = expect_fixed(run(&schema, r#"{"n": "3"}"#));
    assert_eq!(value, json!({"n": 3}));
}

#[test]
fn self_reference_terminates() {
    let schema = json!({
        "type": "object",
        "properties": {"n": {"type": "integer"}, "child": {"$ref": "#"}}
    });
    let (value, _) = expect_fixed(run(&schema, r#"{"n": "1", "child": {"n": "2"}}"#));
    assert_eq!(value, json!({"n": 1, "child": {"n": 2}}));
}

#[test]
fn keeps_unconvertible_value_behind_ref() {
    let schema = json!({
        "$defs": {"N": {"type": "integer"}},
        "type": "object",
        "properties": {"n": {"type": "integer"}, "m": {"$ref": "#/$defs/N"}}
    });
    let value = expect_invalid_untouched(run(&schema, r#"{"m": "x"}"#));
    assert_eq!(value, json!({"m": "x"}));
}

#[test]
fn rejects_remote_ref_schema() {
    let schema = json!({"$ref": "https://example.com/schema.json"});
    assert!(schemafix::Fixer::new(&schema).is_err());
}
