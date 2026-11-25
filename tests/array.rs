mod common;

use common::{expect_fixed, expect_invalid_untouched, run};
use schemafix::FixKind;
use serde_json::json;

#[test]
fn wraps_singleton_and_corrects_item() {
    let schema = json!({"type": "array", "items": {"type": "integer"}});
    let (value, kinds) = expect_fixed(run(&schema, r#""7""#));
    assert_eq!(value, json!([7]));
    assert_eq!(kinds, [FixKind::WrapInArray, FixKind::StringToNumber]);
}

#[test]
fn corrects_each_item() {
    let schema = json!({"type": "array", "items": {"type": "boolean"}});
    let (value, _) = expect_fixed(run(&schema, r#"["true", false, "False"]"#));
    assert_eq!(value, json!([true, false, false]));
}

#[test]
fn does_not_wrap_null() {
    let schema = json!({"type": "array"});
    let value = expect_invalid_untouched(run(&schema, "null"));
    assert_eq!(value, json!(null));
}

#[test]
fn does_not_wrap_value_of_another_allowed_type() {
    let schema = json!({"type": ["array", "string"], "minLength": 5});
    let value = expect_invalid_untouched(run(&schema, r#""abc""#));
    assert_eq!(value, json!("abc"));
}
