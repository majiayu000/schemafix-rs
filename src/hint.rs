//! Render schema violations as text for the next model turn.

use crate::SchemaViolation;

const HEADER: &str = "Your previous output did not match the required JSON schema. \
Fix the problems below and reply with only the corrected JSON value, no prose or code fences.";

pub(crate) fn render(errors: &[SchemaViolation]) -> String {
    let mut text = String::from(HEADER);
    text.push_str("\nProblems (JSON Pointer: error):");
    for error in errors {
        text.push_str(&format!("\n- {}: {}", error.path, error.message));
    }
    text
}
