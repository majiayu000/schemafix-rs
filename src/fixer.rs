//! Pipeline: raw text → syntax repair → schema correction → validation.

use serde_json::Value;

use crate::correct::Corrector;
use crate::fix::{Fix, FixKind, JsonPointer};
use crate::{Outcome, SchemaError, SchemaViolation};

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
    schema: Value,
    validator: jsonschema::Validator,
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
        Ok(Self {
            schema: schema.clone(),
            validator,
            opts,
        })
    }

    /// Run the full pipeline on raw model output.
    pub fn fix(&self, raw: &str) -> Outcome {
        let mut fixes = Vec::new();
        let mut value = match self.parse(raw, &mut fixes) {
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
            let mut corrector = Corrector::new(&self.schema, &self.opts);
            corrector.correct(&mut value, &self.schema, &JsonPointer::root());
            fixes.extend(corrector.into_fixes());
        }
        let errors = self.violations(&value);
        match (errors.is_empty(), fixes.is_empty()) {
            (true, true) => Outcome::Valid { value },
            (true, false) => Outcome::Fixed { value, fixes },
            (false, _) => Outcome::Invalid {
                value: Some(value),
                fixes,
                errors,
            },
        }
    }

    fn parse(&self, raw: &str, fixes: &mut Vec<Fix>) -> Result<Value, String> {
        if let Some(limit) = self.opts.max_input_bytes
            && raw.len() > limit
        {
            return Err(format!("input exceeds {limit} bytes"));
        }
        if let Ok(value) = serde_json::from_str(raw) {
            return Ok(value);
        }
        let repaired = jsonrepair_rs::jsonrepair(raw)
            .map_err(|err| format!("output is not valid JSON and could not be repaired: {err}"))?;
        let value = serde_json::from_str(&repaired)
            .map_err(|err| format!("repaired output is not valid JSON: {err}"))?;
        fixes.push(Fix::replaced(
            &JsonPointer::root(),
            FixKind::SyntaxRepair,
            Value::String(raw.to_owned()),
            Value::String(repaired),
        ));
        Ok(value)
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
