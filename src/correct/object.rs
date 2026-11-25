//! `properties` recursion, `required` defaults and `additionalProperties: false`.

use serde_json::{Map, Value};

use super::{Corrector, refs};
use crate::fix::{Fix, FixKind, JsonPointer};

pub(super) fn correct(
    c: &mut Corrector<'_>,
    value: &mut Value,
    schema: &Value,
    path: &JsonPointer,
) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    let properties = schema.get("properties").and_then(Value::as_object);
    if let Some(properties) = properties {
        for (name, child) in object.iter_mut() {
            if let Some(child_schema) = properties.get(name) {
                c.correct(child, child_schema, &path.child(name));
            }
        }
    }
    if c.opts.fill_defaults {
        fill_defaults(c, object, schema, properties, path);
    }
    if c.opts.drop_additional_properties {
        drop_additional(c, object, schema, properties, path);
    }
}

fn fill_defaults(
    c: &mut Corrector<'_>,
    object: &mut Map<String, Value>,
    schema: &Value,
    properties: Option<&Map<String, Value>>,
    path: &JsonPointer,
) {
    let (Some(required), Some(properties)) =
        (schema.get("required").and_then(Value::as_array), properties)
    else {
        return;
    };
    for name in required.iter().filter_map(Value::as_str) {
        if object.contains_key(name) {
            continue;
        }
        let Some(property) = properties.get(name) else {
            continue;
        };
        let Some(default) = refs::resolved(c.root, property).get("default") else {
            continue;
        };
        object.insert(name.to_owned(), default.clone());
        c.record(Fix {
            path: path.child(name),
            kind: FixKind::FillDefault,
            before: None,
            after: Some(default.clone()),
        });
    }
}

fn drop_additional(
    c: &mut Corrector<'_>,
    object: &mut Map<String, Value>,
    schema: &Value,
    properties: Option<&Map<String, Value>>,
    path: &JsonPointer,
) {
    // patternProperties would need regex matching to decide; leave such
    // objects to validation rather than guess.
    if schema.get("additionalProperties") != Some(&Value::Bool(false))
        || schema.get("patternProperties").is_some()
    {
        return;
    }
    let extra: Vec<String> = object
        .keys()
        .filter(|name| properties.is_none_or(|known| !known.contains_key(*name)))
        .cloned()
        .collect();
    for name in extra {
        if let Some(removed) = object.remove(&name) {
            c.record(Fix {
                path: path.child(&name),
                kind: FixKind::DropAdditionalProperty,
                before: Some(removed),
                after: None,
            });
        }
    }
}
