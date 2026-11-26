mod common;

use common::{expect_fixed, expect_invalid_untouched, run};
use schemafix::Outcome;
use serde_json::json;

#[test]
fn any_of_picks_first_branch_that_validates_after_fix() {
    let schema = json!({"anyOf": [{"type": "boolean"}, {"type": "integer"}]});
    let (value, _) = expect_fixed(run(&schema, r#""12""#));
    assert_eq!(value, json!(12));
}

#[test]
fn any_of_leaves_value_matching_a_branch() {
    let schema = json!({"anyOf": [{"type": "integer"}, {"type": "string"}]});
    assert!(matches!(run(&schema, r#""12""#), Outcome::Valid { .. }));
}

#[test]
fn one_of_corrects_into_matching_object_branch() {
    let schema = json!({"oneOf": [
        {"type": "object", "properties": {"kind": {"const": "a"}, "n": {"type": "integer"}},
         "required": ["kind", "n"]},
        {"type": "object", "properties": {"kind": {"const": "b"}, "s": {"type": "string"}},
         "required": ["kind", "s"]}
    ]});
    let (value, _) = expect_fixed(run(&schema, r#"{"kind": "a", "n": "5"}"#));
    assert_eq!(value, json!({"kind": "a", "n": 5}));
}

#[test]
fn one_of_skips_correction_matching_several_branches() {
    let schema = json!({"oneOf": [{"type": "integer"}, {"type": "number"}]});
    let value = expect_invalid_untouched(run(&schema, r#""5""#));
    assert_eq!(value, json!("5"));
}

#[test]
fn one_of_branch_via_local_ref() {
    let schema = json!({
        "$defs": {"N": {"type": "integer"}},
        "oneOf": [{"$ref": "#/$defs/N"}, {"type": "null"}]
    });
    let (value, _) = expect_fixed(run(&schema, r#""9""#));
    assert_eq!(value, json!(9));
}

#[test]
fn any_of_keeps_value_no_branch_accepts() {
    let schema = json!({"anyOf": [{"type": "integer"}, {"type": "boolean"}]});
    let value = expect_invalid_untouched(run(&schema, r#""maybe""#));
    assert_eq!(value, json!("maybe"));
}

#[test]
fn all_of_applies_every_subschema() {
    let schema = json!({"allOf": [
        {"type": "object", "properties": {"a": {"type": "integer"}}},
        {"properties": {"b": {"type": "boolean"}}, "required": ["b"]}
    ]});
    let (value, _) = expect_fixed(run(&schema, r#"{"a": "1", "b": "true"}"#));
    assert_eq!(value, json!({"a": 1, "b": true}));
}

#[test]
fn all_of_keeps_unconvertible_value() {
    let schema = json!({"allOf": [{"type": "object", "properties": {"a": {"type": "integer"}}}]});
    let value = expect_invalid_untouched(run(&schema, r#"{"a": "one"}"#));
    assert_eq!(value, json!({"a": "one"}));
}

#[test]
fn branch_cache_survives_moving_the_fixer() {
    let schema = json!({"anyOf": [{"type": "boolean"}, {"type": "integer"}]});
    let fixers = vec![schemafix::Fixer::new(&schema).unwrap()];
    let moved = fixers.into_iter().next().unwrap();
    let (value, _) = expect_fixed(moved.fix(r#""12""#));
    assert_eq!(value, json!(12));
}
