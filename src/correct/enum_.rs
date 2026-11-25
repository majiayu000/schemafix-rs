//! Case-insensitive string `enum` matching.

use serde_json::Value;

use super::Corrector;
use crate::fix::{Fix, FixKind, JsonPointer};

pub(super) fn correct(
    c: &mut Corrector<'_>,
    value: &mut Value,
    schema: &Value,
    path: &JsonPointer,
) {
    if let Some(corrected) = matching_enum_string(value, schema) {
        let after = Value::String(corrected);
        let before = std::mem::replace(value, after.clone());
        c.record(Fix::replaced(path, FixKind::EnumCase, before, after));
    }
}

/// The unique enum member equal to `value` ignoring ASCII case. `None` when
/// the value already matches exactly, matches nothing, or matches several
/// members that differ only in case.
fn matching_enum_string(value: &Value, schema: &Value) -> Option<String> {
    let current = value.as_str()?;
    let choices = schema.get("enum")?.as_array()?;
    if choices.iter().any(|choice| choice == value) {
        return None;
    }

    let mut matching = None;
    for choice in choices.iter().filter_map(Value::as_str) {
        if choice.eq_ignore_ascii_case(current) {
            match matching {
                None => matching = Some(choice),
                Some(previous) if previous == choice => {}
                Some(_) => return None,
            }
        }
    }
    matching.map(str::to_owned)
}
