//! Raw model text → JSON value: prose/fence extraction, then syntax repair.

use serde_json::Value;

use crate::fix::{Fix, FixKind, JsonPointer};

const FENCE: &str = "```";

/// Parse `raw`, recording an [`FixKind::ExtractJson`] and/or
/// [`FixKind::SyntaxRepair`] fix when the text needed either.
pub(crate) fn parse(raw: &str, fixes: &mut Vec<Fix>) -> Result<Value, String> {
    if let Ok(value) = serde_json::from_str(raw) {
        return Ok(value);
    }
    let text = match embedded_json(raw) {
        Some(span) => {
            fixes.push(text_fix(FixKind::ExtractJson, raw, span));
            span
        }
        None => raw,
    };
    if let Ok(value) = serde_json::from_str(text) {
        return Ok(value);
    }
    let repaired = jsonrepair_rs::jsonrepair(text)
        .map_err(|err| format!("output is not valid JSON and could not be repaired: {err}"))?;
    let value = serde_json::from_str(&repaired)
        .map_err(|err| format!("repaired output is not valid JSON: {err}"))?;
    fixes.push(text_fix(FixKind::SyntaxRepair, text, &repaired));
    Ok(value)
}

fn text_fix(kind: FixKind, before: &str, after: &str) -> Fix {
    Fix::replaced(
        &JsonPointer::root(),
        kind,
        Value::String(before.to_owned()),
        Value::String(after.to_owned()),
    )
}

/// The JSON-looking part of `raw` when it is wrapped in prose or a code
/// fence: the body of the first fence, otherwise the first bracketed value
/// (to the end of the text when it never closes, i.e. truncated output).
/// `None` when there is nothing to cut away.
fn embedded_json(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    let bracket = trimmed.find(['{', '[']);
    // A fence only counts when it opens before any JSON bracket, so a fence
    // inside a string value is not mistaken for a wrapper.
    let span = match trimmed.find(FENCE) {
        Some(open) if bracket.is_none_or(|b| open < b) => {
            fence_body(&trimmed[open + FENCE.len()..])
        }
        _ => bracket_span(trimmed, bracket?),
    };
    let span = span.trim();
    (!span.is_empty() && span != trimmed).then_some(span)
}

fn fence_body(after_open: &str) -> &str {
    // Skip the info string (e.g. `json`) up to the end of the fence line.
    let body = after_open
        .find('\n')
        .map_or(after_open, |eol| &after_open[eol + 1..]);
    body.find(FENCE).map_or(body, |close| &body[..close])
}

/// From `start` up to the bracket that closes it, skipping brackets inside
/// double-quoted strings; the whole rest when it never closes.
fn bracket_span(text: &str, start: usize) -> &str {
    let rest = &text[start..];
    let (mut depth, mut in_string, mut escaped) = (0usize, false, false);
    for (index, byte) in rest.bytes().enumerate() {
        if in_string {
            match byte {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return &rest[..=index];
                }
            }
            _ => {}
        }
    }
    rest
}

#[cfg(test)]
mod tests {
    use super::embedded_json;

    #[test]
    fn extracts_fence_body_after_prose() {
        let raw = "Here you go:\n```json\n{\"a\": 1}\n```\nThanks!";
        assert_eq!(embedded_json(raw), Some("{\"a\": 1}"));
    }

    #[test]
    fn extracts_bracket_span_from_prose() {
        let raw = "Sure! The arguments are: {\"a\": [1]} Hope this helps.";
        assert_eq!(embedded_json(raw), Some("{\"a\": [1]}"));
    }

    #[test]
    fn keeps_truncated_tail() {
        assert_eq!(embedded_json("Result: {\"a\": 1"), Some("{\"a\": 1"));
    }

    #[test]
    fn keeps_truncated_nested_value_whole() {
        assert_eq!(embedded_json(r#"{"a": {"b": 1}, "c": [{"d": 2"#), None);
    }

    #[test]
    fn cuts_trailing_prose_and_ignores_brackets_in_strings() {
        let raw = r#"{"a": "}]"} -- done"#;
        assert_eq!(embedded_json(raw), Some(r#"{"a": "}]"}"#));
    }

    #[test]
    fn nothing_to_cut_from_bare_json() {
        assert_eq!(embedded_json("  {\"a\": 1,}  "), None);
        assert_eq!(embedded_json("just words"), None);
    }
}
