//! String → null / boolean / number conversions driven by `type`.

use serde_json::{Number, Value};

use super::Corrector;
use crate::fix::{Fix, FixKind, JsonPointer};

pub(super) fn correct(
    c: &mut Corrector<'_>,
    value: &mut Value,
    types: &[&str],
    path: &JsonPointer,
) {
    let Some(text) = value.as_str() else {
        return;
    };
    // A string that the schema accepts as a string is never reinterpreted.
    if types.is_empty() || types.contains(&"string") {
        return;
    }
    let Some((kind, converted)) = convert(text, types) else {
        return;
    };
    let before = std::mem::replace(value, converted.clone());
    c.record(Fix::replaced(path, kind, before, converted));
}

fn convert(text: &str, types: &[&str]) -> Option<(FixKind, Value)> {
    if types.contains(&"null") && text == "null" {
        return Some((FixKind::StringToNull, Value::Null));
    }
    if types.contains(&"boolean") {
        if text.eq_ignore_ascii_case("true") {
            return Some((FixKind::StringToBoolean, Value::Bool(true)));
        }
        if text.eq_ignore_ascii_case("false") {
            return Some((FixKind::StringToBoolean, Value::Bool(false)));
        }
    }
    let wants_number = types.contains(&"number");
    if !wants_number && !types.contains(&"integer") {
        return None;
    }
    let number = text.parse::<Number>().ok()?;
    let integral = number.is_i64() || number.is_u64();
    if integral || (wants_number && numeric_string_round_trips(text, &number)) {
        return Some((FixKind::StringToNumber, Value::Number(number)));
    }
    None
}

/// Whether converting `text` to `number` loses no precision.
fn numeric_string_round_trips(text: &str, number: &Number) -> bool {
    // Only arbitrary_precision can store u128::MAX. In that mode, compare the
    // retained decimal directly, since as_f64() may round an exact Number.
    if Number::from_u128(u128::MAX).is_some() {
        // Parsing can lowercase E and insert an exponent's optional + sign.
        let decimal_spelling = |text: &str| text.replace('E', "e").replace("e+", "e");
        return decimal_spelling(text) == decimal_spelling(&number.to_string());
    }
    // The shortest float representation can hide rounding of large integers.
    // Render integral floats in full; ordinary fractions use JSON's decimal form.
    let rendered = match number.as_f64() {
        Some(value) if value.fract() == 0.0 => format!("{value:.0}"),
        _ => number.to_string(),
    };
    match (normalized_decimal(text), normalized_decimal(&rendered)) {
        (Some(original), Some(converted)) => original == converted,
        _ => false,
    }
}

/// Split a decimal into (signed significant digits, base-10 exponent).
fn normalized_decimal(text: &str) -> Option<(String, i64)> {
    let (mantissa, exponent) = text.split_once(['e', 'E']).unwrap_or((text, "0"));
    let fraction_len = mantissa
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    let negative = mantissa.starts_with('-');
    let digits: String = mantissa
        .trim_start_matches('-')
        .chars()
        .filter(|ch| *ch != '.')
        .collect();
    let significant = digits.trim_start_matches('0').trim_end_matches('0');
    if significant.is_empty() {
        // A zero mantissa is exact regardless of the exponent's magnitude.
        return Some(("0".to_owned(), 0));
    }
    let trailing_zeros = digits.len() - digits.trim_end_matches('0').len();
    let exponent = exponent
        .parse::<i64>()
        .ok()?
        .checked_sub(i64::try_from(fraction_len).ok()?)?
        .checked_add(i64::try_from(trailing_zeros).ok()?)?;
    let signed_digits = if negative {
        format!("-{significant}")
    } else {
        significant.to_owned()
    };
    Some((signed_digits, exponent))
}
