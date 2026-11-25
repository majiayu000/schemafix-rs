//! Singleton wrapping and `items` recursion.

use serde_json::Value;

use super::{Corrector, has_declared_type};
use crate::fix::{Fix, FixKind, JsonPointer};

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
