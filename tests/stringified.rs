mod common;

use common::{expect_fixed, expect_invalid_untouched, run};
use schemafix::{FixKind, Outcome};
use serde_json::json;

fn schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "customer": {
                "type": "object",
                "properties": {"id": {"type": "integer"}},
                "required": ["id"]
            }
        },
        "required": ["customer"]
    })
}

#[test]
fn parses_stringified_object_and_corrects_inside() {
    let (value, kinds) = expect_fixed(run(&schema(), r#"{"customer": "{\"id\": \"7\"}"}"#));
    assert_eq!(value, json!({"customer": {"id": 7}}));
    assert_eq!(
        kinds,
        [FixKind::ParseStringifiedJson, FixKind::StringToNumber]
    );
}

#[test]
fn keeps_string_that_is_not_json() {
    let value = expect_invalid_untouched(run(&schema(), r#"{"customer": "Ada"}"#));
    assert_eq!(value, json!({"customer": "Ada"}));
}

#[test]
fn keeps_stringified_array_where_object_is_wanted() {
    let value = expect_invalid_untouched(run(&schema(), r#"{"customer": "[1]"}"#));
    assert_eq!(value, json!({"customer": "[1]"}));
}

#[test]
fn keeps_string_when_schema_also_accepts_strings() {
    let schema = json!({"type": ["object", "string"]});
    assert!(matches!(
        run(&schema, r#""{\"id\": 1}""#),
        Outcome::Valid { .. }
    ));
}
