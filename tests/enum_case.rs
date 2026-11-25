mod common;

use common::{expect_fixed, expect_invalid_untouched, run};
use schemafix::FixKind;
use serde_json::json;

#[test]
fn matches_enum_ignoring_case() {
    let schema = json!({"enum": ["celsius", "fahrenheit"]});
    let (value, kinds) = expect_fixed(run(&schema, r#""Celsius""#));
    assert_eq!(value, json!("celsius"));
    assert_eq!(kinds, [FixKind::EnumCase]);
}

#[test]
fn keeps_ambiguous_case_match() {
    let schema = json!({"enum": ["Auto", "AUTO"]});
    let value = expect_invalid_untouched(run(&schema, r#""auto""#));
    assert_eq!(value, json!("auto"));
}

#[test]
fn keeps_string_with_no_match() {
    let schema = json!({"enum": ["celsius", "fahrenheit"]});
    let value = expect_invalid_untouched(run(&schema, r#""kelvin""#));
    assert_eq!(value, json!("kelvin"));
}
