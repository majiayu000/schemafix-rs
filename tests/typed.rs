#![cfg(feature = "schemars")]

use schemafix::{FixError, FixKind, parse};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Unit {
    Celsius,
    Fahrenheit,
}

#[derive(Debug, Deserialize, JsonSchema, PartialEq)]
struct Weather {
    city: String,
    days: u8,
    unit: Unit,
}

#[test]
fn parses_and_corrects_into_type() {
    let (weather, fixes) =
        parse::<Weather>(r#"{"city": "Oslo", "days": "2", "unit": "Celsius"}"#).expect("fixable");
    assert_eq!(
        weather,
        Weather {
            city: "Oslo".into(),
            days: 2,
            unit: Unit::Celsius
        }
    );
    let kinds: Vec<_> = fixes.iter().map(|f| f.kind).collect();
    assert_eq!(kinds, [FixKind::StringToNumber, FixKind::EnumCase]);
}

#[test]
fn returns_invalid_with_retry_hint() {
    let err = parse::<Weather>(r#"{"city": "Oslo", "days": "two", "unit": "kelvin"}"#).unwrap_err();
    let FixError::Invalid(outcome) = err else {
        panic!("expected Invalid, got {err:?}");
    };
    let hint = outcome.retry_hint().unwrap();
    assert!(hint.contains("/days") && hint.contains("/unit"), "{hint}");
}
