#![allow(dead_code)]

use schemafix::{FixKind, FixOptions, Fixer, Outcome};
use serde_json::Value;

pub fn run(schema: &Value, raw: &str) -> Outcome {
    Fixer::new(schema).expect("schema compiles").fix(raw)
}

pub fn run_with(schema: &Value, raw: &str, opts: FixOptions) -> Outcome {
    Fixer::with_options(schema, opts)
        .expect("schema compiles")
        .fix(raw)
}

/// Assert `Fixed` and return the value with the kinds of fixes applied.
pub fn expect_fixed(outcome: Outcome) -> (Value, Vec<FixKind>) {
    match outcome {
        Outcome::Fixed { value, fixes } => (value, fixes.iter().map(|f| f.kind).collect()),
        other => panic!("expected Fixed, got {other:?}"),
    }
}

/// Assert `Invalid` with no schema fixes and return the untouched value.
pub fn expect_invalid_untouched(outcome: Outcome) -> Value {
    match outcome {
        Outcome::Invalid {
            value: Some(value),
            fixes,
            errors,
        } => {
            assert!(fixes.is_empty(), "unexpected fixes {fixes:?}");
            assert!(!errors.is_empty());
            value
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
}
