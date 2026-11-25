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
