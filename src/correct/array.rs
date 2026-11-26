//! Singleton wrapping/unwrapping and `items` recursion.

use serde_json::Value;

use super::{Corrector, has_declared_type};
use crate::fix::{Fix, FixKind, JsonPointer};

/// `[x]` where the schema declares types but no array: use `x` itself.
pub(super) fn unwrap_singleton(
    c: &mut Corrector<'_>,
    value: &mut Value,
    types: &[&str],
    path: &JsonPointer,
) {
    let single = value.as_array().is_some_and(|items| items.len() == 1);
    if !single || types.is_empty() || types.contains(&"array") {
        return;
    }
    let before = std::mem::take(value);
    *value = before[0].clone();
    c.record(Fix::replaced(
        path,
        FixKind::UnwrapSingleton,
        before,
        value.clone(),
    ));
}

pub(super) fn correct(
    c: &mut Corrector<'_>,
    value: &mut Value,
    schema: &Value,
    types: &[&str],
    path: &JsonPointer,
) {
    let wrap = types.contains(&"array")
        && !value.is_array()
        && !value.is_null()
        && !has_declared_type(value, types);
    if wrap {
        let before = std::mem::take(value);
        *value = Value::Array(vec![before.clone()]);
        c.record(Fix::replaced(
            path,
            FixKind::WrapInArray,
            before,
            value.clone(),
        ));
    }

    // Tuple-form `items` (an array of schemas) is left to validation.
    let (Some(items), Some(items_schema)) = (value.as_array_mut(), schema.get("items")) else {
        return;
    };
    if !items_schema.is_object() {
        return;
    }
    for (index, item) in items.iter_mut().enumerate() {
        c.correct(item, items_schema, &path.child(index));
    }
}
