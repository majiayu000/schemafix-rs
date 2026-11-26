//! A JSON object/array that arrived as a string, e.g. `"{\"a\": 1}"`.

use serde_json::Value;

use super::{Corrector, has_declared_type};
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
    // Only when the schema wants a container and does not accept the string as-is.
    let wants_container = types.contains(&"object") || types.contains(&"array");
    if !wants_container || types.contains(&"string") {
        return;
    }
    let Ok(parsed) = serde_json::from_str::<Value>(text) else {
        return;
    };
    if !(parsed.is_object() || parsed.is_array()) || !has_declared_type(&parsed, types) {
        return;
    }
    let before = std::mem::replace(value, parsed.clone());
    c.record(Fix::replaced(
        path,
        FixKind::ParseStringifiedJson,
        before,
        parsed,
    ));
}
