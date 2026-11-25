//! Local `$ref` resolution (`#`, `#/$defs/...`, `#/definitions/...`).
//! Remote references are not resolved.

use serde_json::Value;

use super::MAX_REF_DEPTH;

/// The schema a local `$ref` points to, if `schema` has one.
pub(super) fn target<'a>(root: &'a Value, schema: &Value) -> Option<&'a Value> {
    let reference = schema.get("$ref")?.as_str()?;
    let pointer = reference.strip_prefix('#')?;
    if pointer.is_empty() {
        return Some(root);
    }
    root.pointer(pointer)
}

/// Follow a chain of local `$ref`s to the final schema.
pub(super) fn resolved<'a>(root: &'a Value, schema: &'a Value) -> &'a Value {
    let mut current = schema;
    for _ in 0..MAX_REF_DEPTH {
        match target(root, current) {
            Some(next) => current = next,
            None => break,
        }
    }
    current
}
