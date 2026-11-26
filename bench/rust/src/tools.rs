//! The Rust pipelines under test.

use std::collections::HashMap;

use rstructor::{Instructor, LLMClient, MockClient};
use schemafix::{FixOptions, Fixer, Outcome};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::Case;
use crate::typed::{Event, Order, Weather};

pub enum Runner {
    /// schemafix with every correction enabled (drop_additional_properties on).
    Schemafix(HashMap<String, Fixer>),
    /// jsonrepair-rs syntax repair + jsonschema validation, no schema correction.
    JsonrepairJsonschema(HashMap<String, jsonschema::Validator>),
    /// rstructor's full offline materialize path via MockClient (retries = 0).
    RstructorMock(tokio::runtime::Runtime),
    /// rstructor's decode step alone: serde_path_to_error + Instructor::validate,
    /// mirroring rstructor 0.6.0 src/decode.rs + backend/utils.rs.
    RstructorDecode,
}

impl Runner {
    pub fn new(tool: &str, schemas: &HashMap<String, Value>) -> Self {
        match tool {
            "schemafix" => {
                let opts = FixOptions { drop_additional_properties: true, ..FixOptions::default() };
                Self::Schemafix(compile(schemas, |s| Fixer::with_options(s, opts.clone()).unwrap()))
            }
            "jsonrepair_jsonschema" => Self::JsonrepairJsonschema(compile(schemas, |s| {
                jsonschema::validator_for(s).unwrap()
            })),
            "rstructor_mock" => Self::RstructorMock(
                tokio::runtime::Builder::new_current_thread().build().unwrap(),
            ),
            "rstructor_decode" => Self::RstructorDecode,
            other => panic!("unknown tool {other}"),
        }
    }

    /// Crate names whose versions apply (resolved from Cargo.lock by score.py).
    pub fn versions(&self) -> Vec<&'static str> {
        match self {
            Self::Schemafix(_) => vec!["schemafix-rs", "jsonrepair-rs", "jsonschema"],
            Self::JsonrepairJsonschema(_) => vec!["jsonrepair-rs", "jsonschema"],
            Self::RstructorMock(_) | Self::RstructorDecode => vec!["rstructor", "serde_json"],
        }
    }

    pub fn run(&self, case: &Case) -> Option<Value> {
        match self {
            Self::Schemafix(fixers) => match fixers[&case.schema].fix(&case.raw) {
                Outcome::Valid { value } | Outcome::Fixed { value, .. } => Some(value),
                Outcome::Invalid { .. } => None,
            },
            Self::JsonrepairJsonschema(validators) => {
                let text = jsonrepair_rs::jsonrepair(&case.raw).ok()?;
                let value: Value = serde_json::from_str(&text).ok()?;
                validators[&case.schema].is_valid(&value).then_some(value)
            }
            Self::RstructorMock(rt) => match case.schema.as_str() {
                "weather" => mock::<Weather>(rt, &case.raw),
                "order" => mock::<Order>(rt, &case.raw),
                _ => mock::<Event>(rt, &case.raw),
            },
            Self::RstructorDecode => match case.schema.as_str() {
                "weather" => decode::<Weather>(&case.raw),
                "order" => decode::<Order>(&case.raw),
                _ => decode::<Event>(&case.raw),
            },
        }
    }
}

fn compile<T>(schemas: &HashMap<String, Value>, f: impl Fn(&Value) -> T) -> HashMap<String, T> {
    schemas.iter().map(|(k, v)| (k.clone(), f(v))).collect()
}

fn mock<T>(rt: &tokio::runtime::Runtime, raw: &str) -> Option<Value>
where
    T: Instructor + DeserializeOwned + Serialize + Send + 'static,
{
    let client = MockClient::new().with_response(raw).with_retries(0);
    let typed: T = rt.block_on(client.materialize("benchmark")).ok()?;
    serde_json::to_value(typed).ok()
}

fn decode<T: Instructor + DeserializeOwned + Serialize>(raw: &str) -> Option<Value> {
    let mut de = serde_json::Deserializer::from_str(raw);
    let typed: T = serde_path_to_error::deserialize(&mut de).ok()?;
    de.end().ok()?;
    typed.validate().ok()?;
    serde_json::to_value(typed).ok()
}
