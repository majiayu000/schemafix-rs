mod common;

use common::{expect_fixed, expect_invalid_untouched, run, run_with};
use schemafix::{FixKind, FixOptions};
use serde_json::json;

fn with_default() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "unit": {"type": "string", "default": "celsius"}
        },
        "required": ["name", "unit"]
    })
}

#[test]
fn fills_required_default() {
    let (value, kinds) = expect_fixed(run(&with_default(), r#"{"name": "x"}"#));
    assert_eq!(value, json!({"name": "x", "unit": "celsius"}));
    assert_eq!(kinds, [FixKind::FillDefault]);
}

#[test]
fn does_not_fill_when_disabled() {
    let opts = FixOptions {
        fill_defaults: false,
        ..FixOptions::default()
    };
    let value = expect_invalid_untouched(run_with(&with_default(), r#"{"name": "x"}"#, opts));
    assert_eq!(value, json!({"name": "x"}));
}

#[test]
fn does_not_invent_value_without_default() {
    let value = expect_invalid_untouched(run(&with_default(), r#"{"unit": "kelvin"}"#));
    assert_eq!(value, json!({"unit": "kelvin"}));
}

#[test]
fn does_not_fill_optional_property() {
    let schema = json!({
        "type": "object",
        "properties": {"a": {"type": "integer"}, "b": {"default": 1}},
        "required": ["a"]
    });
    let (value, _) = expect_fixed(run(&schema, r#"{"a": "2"}"#));
    assert_eq!(value, json!({"a": 2}));
}

fn closed() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {"a": {"type": "integer"}},
        "additionalProperties": false
    })
}

fn dropping() -> FixOptions {
    FixOptions {
        drop_additional_properties: true,
        ..FixOptions::default()
    }
}

#[test]
fn drops_additional_property_when_enabled() {
    let (value, kinds) = expect_fixed(run_with(
        &closed(),
        r#"{"a": 1, "extra": true}"#,
        dropping(),
    ));
    assert_eq!(value, json!({"a": 1}));
    assert_eq!(kinds, [FixKind::DropAdditionalProperty]);
}

#[test]
fn keeps_additional_property_by_default() {
    let value = expect_invalid_untouched(run(&closed(), r#"{"a": 1, "extra": true}"#));
    assert_eq!(value, json!({"a": 1, "extra": true}));
}

#[test]
fn keeps_property_when_pattern_properties_present() {
    let mut schema = closed();
    schema["patternProperties"] = json!({"^x-": {}});
    let value = expect_invalid_untouched(run_with(&schema, r#"{"a": 1, "extra": 2}"#, dropping()));
    assert_eq!(value, json!({"a": 1, "extra": 2}));
}

#[test]
fn fix_paths_are_json_pointers() {
    let schema = json!({
        "type": "object",
        "properties": {"a/b": {"type": "array", "items": {"type": "integer"}}}
    });
    let outcome = run(&schema, r#"{"a/b": ["1"]}"#);
    assert_eq!(outcome.fixes()[0].path.as_str(), "/a~1b/0");
}
