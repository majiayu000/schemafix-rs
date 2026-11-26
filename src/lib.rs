//! Validate, repair and correct LLM structured output against a JSON Schema.
//!
//! Pipeline: raw text → [`jsonrepair_rs`] syntax repair → conservative
//! schema-guided corrections (each recorded as a [`Fix`]) → [`jsonschema`]
//! validation → [`Outcome`]. When validation still fails,
//! [`Outcome::retry_hint`] renders the errors as text to send back to the model.
//!
//! ```
//! use schemafix::{Fixer, Outcome};
//! use serde_json::json;
//!
//! let schema = json!({
//!     "type": "object",
//!     "properties": { "count": { "type": "integer" } },
//!     "required": ["count"]
//! });
//! let fixer = Fixer::new(&schema).unwrap();
//! match fixer.fix(r#"{"count": "3"}"#) {
//!     Outcome::Fixed { value, fixes } => {
//!         assert_eq!(value, json!({ "count": 3 }));
//!         assert_eq!(fixes.len(), 1);
//!     }
//!     other => panic!("unexpected {other:?}"),
//! }
//! ```

mod correct;
mod fix;
mod fixer;
mod hint;
mod parse;
#[cfg(feature = "schemars")]
mod typed;

use std::fmt;

use serde_json::Value;

pub use fix::{Fix, FixKind, JsonPointer};
pub use fixer::{FixOptions, Fixer};
#[cfg(feature = "schemars")]
pub use typed::{FixError, parse};

/// Result of running the pipeline on one model output.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The raw text was valid JSON that matched the schema unchanged.
    Valid { value: Value },
    /// The value matches the schema after the listed fixes.
    Fixed { value: Value, fixes: Vec<Fix> },
    /// The value still fails validation. `value` is `None` when the text could
    /// not be parsed as JSON at all.
    Invalid {
        value: Option<Value>,
        fixes: Vec<Fix>,
        errors: Vec<SchemaViolation>,
    },
}

impl Outcome {
    /// Prompt text describing every violation; `None` unless `Invalid`.
    pub fn retry_hint(&self) -> Option<String> {
        match self {
            Self::Invalid { errors, .. } => Some(hint::render(errors)),
            _ => None,
        }
    }

    /// The final value, if one was produced.
    pub fn value(&self) -> Option<&Value> {
        match self {
            Self::Valid { value } | Self::Fixed { value, .. } => Some(value),
            Self::Invalid { value, .. } => value.as_ref(),
        }
    }

    /// Fixes applied; empty for `Valid`.
    pub fn fixes(&self) -> &[Fix] {
        match self {
            Self::Valid { .. } => &[],
            Self::Fixed { fixes, .. } | Self::Invalid { fixes, .. } => fixes,
        }
    }
}

/// One validation error at an instance location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaViolation {
    pub path: JsonPointer,
    pub message: String,
}

impl fmt::Display for SchemaViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// The schema itself could not be compiled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError(pub String);

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid schema: {}", self.0)
    }
}

impl std::error::Error for SchemaError {}
