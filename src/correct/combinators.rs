//! `oneOf` / `anyOf`: keep a value that already matches a branch, otherwise
//! take the first branch whose corrected value validates.

use serde_json::Value;

use super::{Corrector, refs};
use crate::fix::JsonPointer;

pub(super) fn correct(
    c: &mut Corrector<'_>,
    value: &mut Value,
    schema: &Value,
    path: &JsonPointer,
) {
    for (keyword, exactly_one) in [("anyOf", false), ("oneOf", true)] {
        if let Some(branches) = schema.get(keyword).and_then(Value::as_array) {
            correct_branches(c, value, branches, exactly_one, path);
        }
    }
}

fn correct_branches(
    c: &mut Corrector<'_>,
    value: &mut Value,
    branches: &[Value],
    exactly_one: bool,
    path: &JsonPointer,
) {
    let checks: Vec<Option<jsonschema::Validator>> = branches
        .iter()
        .map(|branch| compile(c.root, branch))
        .collect();
    // Any branch that already accepts the value means there is nothing to fix;
    // several accepting branches of a oneOf is an ambiguity left to validation.
    if checks.iter().flatten().any(|check| check.is_valid(value)) {
        return;
    }
    for (branch, check) in branches.iter().zip(&checks) {
        let Some(check) = check else {
            continue;
        };
        let mut candidate = value.clone();
        let mut trial = c.trial();
        trial.correct(&mut candidate, branch, path);
        if !check.is_valid(&candidate) {
            continue;
        }
        if exactly_one && matching_count(&checks, &candidate) != 1 {
            continue;
        }
        *value = candidate;
        for fix in trial.into_fixes() {
            c.record(fix);
        }
        return;
    }
}

fn matching_count(checks: &[Option<jsonschema::Validator>], value: &Value) -> usize {
    checks
        .iter()
        .flatten()
        .filter(|check| check.is_valid(value))
        .count()
}

/// Compile a branch as a standalone schema, carrying the root's dialect and
/// local definitions so `#/$defs/...` references still resolve. A branch
/// that cannot be compiled on its own is skipped.
fn compile(root: &Value, branch: &Value) -> Option<jsonschema::Validator> {
    let branch = refs::resolved(root, branch);
    let mut standalone = branch.clone();
    if let (Some(object), Some(root_object)) = (standalone.as_object_mut(), root.as_object()) {
        for key in ["$schema", "$defs", "definitions"] {
            if let Some(shared) = root_object.get(key) {
                object.entry(key).or_insert_with(|| shared.clone());
            }
        }
    }
    jsonschema::validator_for(&standalone).ok()
}
