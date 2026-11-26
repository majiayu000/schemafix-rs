//! Schema-guided corrections. Every correction is conservative: when the
//! schema allows more than one interpretation the value is left unchanged and
//! final validation reports it.

mod array;
mod combinators;
mod enum_;
mod object;
mod refs;
mod scalar;
mod stringified;

pub(crate) use combinators::Branches;

use serde_json::Value;

use crate::fix::{Fix, JsonPointer};
use crate::fixer::FixOptions;

/// Maximum number of `$ref` / `allOf` hops followed without descending into the value.
const MAX_REF_DEPTH: usize = 32;

pub(crate) struct Corrector<'a> {
    root: &'a Value,
    opts: &'a FixOptions,
    branches: &'a Branches,
    fixes: Vec<Fix>,
}

impl<'a> Corrector<'a> {
    pub(crate) fn new(root: &'a Value, opts: &'a FixOptions, branches: &'a Branches) -> Self {
        Self {
            root,
            opts,
            branches,
            fixes: Vec::new(),
        }
    }

    pub(crate) fn into_fixes(self) -> Vec<Fix> {
        self.fixes
    }

    pub(crate) fn correct(&mut self, value: &mut Value, schema: &Value, path: &JsonPointer) {
        self.correct_at_depth(value, schema, path, 0);
    }

    fn correct_at_depth(
        &mut self,
        value: &mut Value,
        schema: &Value,
        path: &JsonPointer,
        ref_depth: usize,
    ) {
        if let Some(target) = refs::target(self.root, schema)
            && ref_depth < MAX_REF_DEPTH
        {
            self.correct_at_depth(value, target, path, ref_depth + 1);
        }
        let types = declared_types(schema);
        array::unwrap_singleton(self, value, &types, path);
        stringified::correct(self, value, &types, path);
        scalar::correct(self, value, &types, path);
        array::correct(self, value, schema, &types, path);
        object::correct(self, value, schema, path);
        enum_::correct(self, value, schema, path);
        combinators::correct(self, value, schema, path);
        // allOf: every subschema applies, so apply each one's corrections in turn.
        if let Some(subschemas) = schema.get("allOf").and_then(Value::as_array)
            && ref_depth < MAX_REF_DEPTH
        {
            for subschema in subschemas {
                self.correct_at_depth(value, subschema, path, ref_depth + 1);
            }
        }
    }

    fn record(&mut self, fix: Fix) {
        self.fixes.push(fix);
    }

    /// A fresh corrector that shares the schema root and options, used to try
    /// a combinator branch without committing its fixes.
    fn trial(&self) -> Corrector<'a> {
        Corrector::new(self.root, self.opts, self.branches)
    }
}

/// The `type` keyword as a list; empty when absent.
fn declared_types(schema: &Value) -> Vec<&str> {
    match schema.get("type") {
        Some(Value::String(name)) => vec![name.as_str()],
        Some(Value::Array(names)) => names.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    }
}

/// Whether `value` already has one of the declared JSON types.
fn has_declared_type(value: &Value, types: &[&str]) -> bool {
    types.iter().any(|name| match *name {
        "null" => value.is_null(),
        "boolean" => value.is_boolean(),
        "string" => value.is_string(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "number" => value.is_number(),
        "integer" => value.as_f64().is_some_and(|number| number.fract() == 0.0),
        _ => false,
    })
}
