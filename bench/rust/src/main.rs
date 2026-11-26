//! Rust competitors. Writes <out>/<tool>.jsonl and <out>/<tool>.timing.json.
//!   cargo run --release -- <tool> <corpus.jsonl> <schemas-dir> <out-dir> <reps>

mod tools;
mod typed;

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

use serde_json::{Value, json};

pub struct Case {
    pub id: String,
    pub schema: String,
    pub raw: String,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [tool, corpus, schemas, out, reps] = args.as_slice() else {
        panic!("usage: <tool> <corpus> <schemas> <out> <reps>");
    };
    let reps: usize = reps.parse().expect("reps");
    let cases: Vec<Case> = fs::read_to_string(corpus)
        .expect("corpus")
        .lines()
        .map(|line| {
            let v: Value = serde_json::from_str(line).expect("case");
            let s = |k: &str| v[k].as_str().expect(k).to_owned();
            Case { id: s("id"), schema: s("schema"), raw: s("raw") }
        })
        .collect();
    let mut schema_map = HashMap::new();
    for entry in fs::read_dir(schemas).expect("schemas") {
        let path = entry.expect("entry").path();
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let schema: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        schema_map.insert(name, schema);
    }
    let runner = tools::Runner::new(tool, &schema_map);

    let outputs: Vec<Option<Value>> = cases.iter().map(|c| runner.run(c)).collect();
    let mut samples = Vec::with_capacity(cases.len() * reps);
    for _ in 0..reps {
        for case in &cases {
            let start = Instant::now();
            std::hint::black_box(runner.run(case));
            samples.push(start.elapsed().as_nanos() as u64);
        }
    }

    let out = Path::new(out);
    fs::create_dir_all(out).unwrap();
    let mut file = fs::File::create(out.join(format!("{tool}.jsonl"))).unwrap();
    for (case, output) in cases.iter().zip(&outputs) {
        writeln!(file, "{}", json!({"id": case.id, "output": output})).unwrap();
    }
    let timing = json!({
        "tool": tool,
        "language": "rust",
        "versions": runner.versions(),
        "reps": reps,
        "bytes_per_pass": cases.iter().map(|c| c.raw.len()).sum::<usize>(),
        "samples_ns": samples,
    });
    fs::write(out.join(format!("{tool}.timing.json")), timing.to_string()).unwrap();
}
