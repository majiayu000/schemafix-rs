//! `oneOf` / `anyOf`: keep a value that already matches a branch, otherwise
//! take the first branch whose corrected value validates.

use std::collections::HashMap;

use serde_json::Value;

use super::{Corrector, refs};
use crate::fix::JsonPointer;

const KEYWORDS: [(&str, bool); 2] = [("anyOf", false), ("oneOf", true)];

/// Validators for every `anyOf`/`oneOf` branch in a schema, compiled once.
/// Keyed by the branch node's address inside the (boxed, never mutated)
/// root schema; `None` marks a branch that cannot be compiled on its own.
pub(crate) struct Branches(HashMap<usize, Option<jsonschema::Validator>>);

impl Branches {
    pub(crate) fn compile(root: &Value) -> Self {
        let mut found = HashMap::new();
        collect(root, root, &mut found);
        Self(found)
    }

    fn get(&self, branch: &Value) -> Option<&jsonschema::Validator> {
        self.0.get(&address(branch))?.as_ref()
    }
}

fn address(node: &Value) -> usize {
    std::ptr::from_ref(node) as usize
}

fn collect(root: &Value, node: &Value, found: &mut HashMap<usize, Option<jsonschema::Validator>>) {
    match node {
        Value::Object(map) => {
            for (keyword, _) in KEYWORDS {
                for branch in map
                    .get(keyword)
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    found.insert(address(branch), standalone(root, branch));
                }
            }
            map.values().for_each(|child| collect(root, child, found));
        }
        Value::Array(items) => items.iter().for_each(|child| collect(root, child, found)),
        _ => {}
    }
}

pub(super) fn correct(
    c: &mut Corrector<'_>,
    value: &mut Value,
    schema: &Value,
    path: &JsonPointer,
) {
    for (keyword, exactly_one) in KEYWORDS {
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
    let checks: Vec<_> = branches
        .iter()
        .map(|branch| c.branches.get(branch))
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
        if exactly_one
            && checks
                .iter()
                .flatten()
                .filter(|v| v.is_valid(&candidate))
                .count()
                != 1
        {
            continue;
        }
        *value = candidate;
        for fix in trial.into_fixes() {
            c.record(fix);
        }
        return;
    }
}

/// Compile a branch as a standalone schema, carrying the root's dialect and
/// local definitions so `#/$defs/...` references still resolve.
fn standalone(root: &Value, branch: &Value) -> Option<jsonschema::Validator> {
    let mut schema = refs::resolved(root, branch).clone();
    if let (Some(object), Some(root_object)) = (schema.as_object_mut(), root.as_object()) {
        for key in ["$schema", "$defs", "definitions"] {
            if let Some(shared) = root_object.get(key) {
                object.entry(key).or_insert_with(|| shared.clone());
            }
        }
    }
    jsonschema::validator_for(&schema).ok()
}
