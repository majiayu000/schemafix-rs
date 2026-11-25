//! Audit records for every change made to the model output.

use std::fmt;

use serde_json::Value;

/// An RFC 6901 JSON Pointer into the instance (`""` is the root).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct JsonPointer(String);

impl JsonPointer {
    /// The pointer to the document root.
    pub fn root() -> Self {
        Self::default()
    }

    /// Return a pointer to a child property or array index.
    pub fn child(&self, token: impl fmt::Display) -> Self {
        let token = token.to_string().replace('~', "~0").replace('/', "~1");
        Self(format!("{}/{}", self.0, token))
    }

    /// The pointer text, e.g. `/items/0/name`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for JsonPointer {
    fn from(text: &str) -> Self {
        Self(text.to_owned())
    }
}

impl fmt::Display for JsonPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            f.write_str("(root)")
        } else {
            f.write_str(&self.0)
        }
    }
}

/// What kind of correction was applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FixKind {
    /// The raw text was not valid JSON and was repaired by jsonrepair-rs.
    SyntaxRepair,
    /// A numeric string was converted to a number or integer.
    StringToNumber,
    /// `"true"` / `"false"` (any ASCII case) was converted to a boolean.
    StringToBoolean,
    /// The string `"null"` was converted to `null`.
    StringToNull,
    /// A string was replaced by the unique enum member that matches it ignoring ASCII case.
    EnumCase,
    /// A single value was wrapped in a one-element array.
    WrapInArray,
    /// A missing required property was filled from its schema `default`.
    FillDefault,
    /// A property forbidden by `additionalProperties: false` was removed.
    DropAdditionalProperty,
}

impl fmt::Display for FixKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::SyntaxRepair => "syntax_repair",
            Self::StringToNumber => "string_to_number",
            Self::StringToBoolean => "string_to_boolean",
            Self::StringToNull => "string_to_null",
            Self::EnumCase => "enum_case",
            Self::WrapInArray => "wrap_in_array",
            Self::FillDefault => "fill_default",
            Self::DropAdditionalProperty => "drop_additional_property",
        };
        f.write_str(name)
    }
}

/// One change made to the value. `before` is `None` for inserted values,
/// `after` is `None` for removed values. For [`FixKind::SyntaxRepair`] both
/// hold the raw and repaired text as JSON strings.
#[derive(Debug, Clone, PartialEq)]
pub struct Fix {
    pub path: JsonPointer,
    pub kind: FixKind,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

impl Fix {
    pub(crate) fn replaced(path: &JsonPointer, kind: FixKind, before: Value, after: Value) -> Self {
        Self {
            path: path.clone(),
            kind,
            before: Some(before),
            after: Some(after),
        }
    }
}

impl fmt::Display for Fix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let show = |v: &Option<Value>| v.as_ref().map_or("<none>".to_owned(), Value::to_string);
        write!(
            f,
            "{} {}: {} -> {}",
            self.path,
            self.kind,
            show(&self.before),
            show(&self.after)
        )
    }
}
