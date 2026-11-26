//! Pipeline: raw text → extraction + syntax repair → schema correction → validation.

use serde_json::Value;

use crate::correct::{Branches, Corrector};
use crate::fix::JsonPointer;
use crate::{Outcome, SchemaError, SchemaViolation, parse};

/// Options controlling which corrections are allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixOptions {
    /// Remove properties rejected by `additionalProperties: false`. Default `false`.
    pub drop_additional_properties: bool,
    /// Fill missing `required` properties that declare a `default`. Default `true`.
    pub fill_defaults: bool,
    /// Reject inputs larger than this many bytes before parsing. Default unlimited.
    pub max_input_bytes: Option<usize>,
}

impl Default for FixOptions {
    fn default() -> Self {
        Self {
            drop_additional_properties: false,
            fill_defaults: true,
            max_input_bytes: None,
        }
    }
}

/// A compiled schema plus options; reusable across many inputs.
pub struct Fixer {
    // Boxed so node addresses (the keys of `branches`) stay put when the
    // Fixer moves.
    schema: Box<Value>,
    validator: jsonschema::Validator,
    branches: Branches,
    opts: FixOptions,
}

impl Fixer {
    /// Compile `schema` with default options.
    pub fn new(schema: &Value) -> Result<Self, SchemaError> {
        Self::with_options(schema, FixOptions::default())
    }

    /// Compile `schema` with explicit options. Remote `$ref`s are not fetched
    /// and make compilation fail.
    pub fn with_options(schema: &Value, opts: FixOptions) -> Result<Self, SchemaError> {
        let validator =
            jsonschema::validator_for(schema).map_err(|err| SchemaError(err.to_string()))?;
        let schema = Box::new(schema.clone());
        let branches = Branches::compile(&schema);
        Ok(Self {
            schema,
            validator,
            branches,
            opts,
        })
    }

    /// Run the full pipeline on raw model output.
    pub fn fix(&self, raw: &str) -> Outcome {
        let mut fixes = Vec::new();
        let parsed = self
            .check_size(raw)
            .and_then(|()| parse::parse(raw, &mut fixes));
        let mut value = match parsed {
            Ok(value) => value,
            Err(message) => {
                return Outcome::Invalid {
                    value: None,
                    fixes,
                    errors: vec![SchemaViolation {
                        path: JsonPointer::root(),
                        message,
                    }],
                };
            }
        };
        // A value that already validates is never touched by schema corrections.
        if !self.validator.is_valid(&value) {
            let mut corrector = Corrector::new(&self.schema, &self.opts, &self.branches);
            corrector.correct(&mut value, &self.schema, &JsonPointer::root());
            fixes.extend(corrector.into_fixes());
            if !self.validator.is_valid(&value) {
                let errors = self.violations(&value);
                return Outcome::Invalid {
                    value: Some(value),
                    fixes,
                    errors,
                };
            }
        }
        if fixes.is_empty() {
            Outcome::Valid { value }
        } else {
            Outcome::Fixed { value, fixes }
        }
    }

    fn check_size(&self, raw: &str) -> Result<(), String> {
        match self.opts.max_input_bytes {
            Some(limit) if raw.len() > limit => Err(format!("input exceeds {limit} bytes")),
            _ => Ok(()),
        }
    }

    fn violations(&self, value: &Value) -> Vec<SchemaViolation> {
        self.validator
            .iter_errors(value)
            .map(|err| SchemaViolation {
                path: JsonPointer::from(err.instance_path().as_str()),
                message: err.to_string(),
            })
            .collect()
    }
}
