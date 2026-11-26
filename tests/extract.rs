mod common;

use common::{expect_fixed, run};
use schemafix::{FixKind, Outcome};
use serde_json::json;

fn schema() -> serde_json::Value {
    json!({"type": "object", "properties": {"n": {"type": "integer"}}, "required": ["n"]})
}

#[test]
fn extracts_json_from_prose_and_fence() {
    let raw = "Here is the result:\n```json\n{\"n\": 3}\n```\nLet me know!";
    let (value, kinds) = expect_fixed(run(&schema(), raw));
    assert_eq!(value, json!({"n": 3}));
    assert_eq!(kinds, [FixKind::ExtractJson]);
}

#[test]
fn extracts_inline_json_then_repairs_and_corrects() {
    let raw = "Sure! The arguments are: {n: '3',} Hope this helps.";
    let (value, kinds) = expect_fixed(run(&schema(), raw));
    assert_eq!(value, json!({"n": 3}));
    assert_eq!(
        kinds,
        [
            FixKind::ExtractJson,
            FixKind::SyntaxRepair,
            FixKind::StringToNumber
        ]
    );
}

#[test]
fn prose_without_json_is_invalid() {
    let outcome = run(&schema(), "I could not find that information.");
    assert!(matches!(
        outcome,
        Outcome::Invalid { value: None, .. } | Outcome::Invalid { .. }
    ));
    assert!(
        outcome
            .fixes()
            .iter()
            .all(|f| f.kind != FixKind::ExtractJson)
    );
}

#[test]
fn fence_inside_a_string_value_is_not_a_wrapper() {
    let schema = json!({"type": "object", "properties": {"note": {"type": "string"}}});
    let (value, kinds) = expect_fixed(run(&schema, r#"{"note": "use ``` fences",}"#));
    assert_eq!(value, json!({"note": "use ``` fences"}));
    assert_eq!(kinds, [FixKind::SyntaxRepair]);
}

#[test]
fn truncated_nested_output_is_not_cut_short() {
    let schema = json!({"type": "object", "properties": {
        "a": {"type": "object"}, "b": {"type": "array", "items": {"type": "integer"}}
    }, "required": ["a", "b"]});
    let (value, kinds) = expect_fixed(run(&schema, r#"{"a": {"x": 1}, "b": [1, "2""#));
    assert_eq!(value, json!({"a": {"x": 1}, "b": [1, 2]}));
    assert_eq!(kinds, [FixKind::SyntaxRepair, FixKind::StringToNumber]);
}
