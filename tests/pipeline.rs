mod common;

use common::{expect_fixed, run, run_with};
use schemafix::{FixKind, FixOptions, Outcome};
use serde_json::json;

fn schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {"city": {"type": "string"}, "days": {"type": "integer"}},
        "required": ["city", "days"]
    })
}

#[test]
fn valid_input_passes_unchanged() {
    let outcome = run(&schema(), r#"{"city": "Paris", "days": 3}"#);
    assert_eq!(
        outcome,
        Outcome::Valid {
            value: json!({"city": "Paris", "days": 3})
        }
    );
    assert!(outcome.retry_hint().is_none());
}

#[test]
fn syntax_repair_is_recorded() {
    let raw = "```json\n{city: 'Paris', days: 3,}\n```";
    let (value, kinds) = expect_fixed(run(&schema(), raw));
    assert_eq!(value, json!({"city": "Paris", "days": 3}));
    assert_eq!(kinds, [FixKind::ExtractJson, FixKind::SyntaxRepair]);
}

#[test]
fn syntax_and_schema_fixes_combine() {
    let (value, kinds) = expect_fixed(run(&schema(), r#"{"city": "Paris", "days": "3""#));
    assert_eq!(value, json!({"city": "Paris", "days": 3}));
    assert_eq!(kinds, [FixKind::SyntaxRepair, FixKind::StringToNumber]);
}

#[test]
fn valid_value_is_not_corrected() {
    let schema = json!({"type": "object", "properties": {"x": {}}, "additionalProperties": false});
    let opts = FixOptions {
        drop_additional_properties: true,
        ..FixOptions::default()
    };
    assert!(matches!(
        run_with(&schema, r#"{"x": "1"}"#, opts),
        Outcome::Valid { .. }
    ));
}

#[test]
fn invalid_reports_errors_and_retry_hint() {
    let outcome = run(&schema(), r#"{"city": "Paris", "days": "three"}"#);
    let Outcome::Invalid { value, errors, .. } = &outcome else {
        panic!("expected Invalid, got {outcome:?}");
    };
    assert_eq!(
        value.as_ref(),
        Some(&json!({"city": "Paris", "days": "three"}))
    );
    assert_eq!(errors[0].path.as_str(), "/days");
    let hint = outcome.retry_hint().unwrap();
    assert!(hint.contains("- /days: "), "{hint}");
}

#[test]
fn missing_required_is_reported_at_root() {
    let hint = run(&schema(), r#"{"city": "Paris"}"#).retry_hint().unwrap();
    assert!(hint.contains("(root): "), "{hint}");
    assert!(hint.contains("days"), "{hint}");
}

#[test]
fn oversized_input_is_rejected_before_parsing() {
    let opts = FixOptions {
        max_input_bytes: Some(4),
        ..FixOptions::default()
    };
    let outcome = run_with(&schema(), r#"{"city": "Paris", "days": 3}"#, opts);
    assert!(matches!(outcome, Outcome::Invalid { value: None, .. }));
    assert!(outcome.retry_hint().unwrap().contains("exceeds 4 bytes"));
}

#[test]
fn invalid_schema_is_an_error() {
    assert!(schemafix::Fixer::new(&json!({"type": 12})).is_err());
}
