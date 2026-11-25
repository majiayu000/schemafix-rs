//! `schemafix --schema s.json < llm_output.txt`
//!
//! Exit codes: 0 valid as-is, 1 fixed, 2 still invalid, 3 input/schema/usage error.

use std::io::Read;
use std::process::ExitCode;

use schemafix::{FixOptions, Fixer, Outcome};
use serde_json::Value;

const USAGE: &str = "usage: schemafix --schema <schema.json> [--drop-additional] [--no-defaults] \
[--max-input-bytes <n>] < input

Reads LLM output on stdin, writes the corrected JSON on stdout,
fixes and retry hints on stderr.

exit codes: 0 valid, 1 fixed, 2 invalid, 3 input/schema/usage error";

const EXIT_VALID: u8 = 0;
const EXIT_FIXED: u8 = 1;
const EXIT_INVALID: u8 = 2;
const EXIT_ERROR: u8 = 3;

struct Args {
    schema_path: String,
    opts: FixOptions,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("schemafix: {message}");
            ExitCode::from(EXIT_ERROR)
        }
    }
}

fn run() -> Result<u8, String> {
    let Some(args) = parse_args(std::env::args().skip(1))? else {
        println!("{USAGE}");
        return Ok(EXIT_VALID);
    };
    let schema_text = std::fs::read_to_string(&args.schema_path)
        .map_err(|err| format!("cannot read schema {}: {err}", args.schema_path))?;
    let schema: Value = serde_json::from_str(&schema_text)
        .map_err(|err| format!("schema {} is not JSON: {err}", args.schema_path))?;
    let fixer = Fixer::with_options(&schema, args.opts).map_err(|err| err.to_string())?;

    let mut raw = String::new();
    std::io::stdin()
        .read_to_string(&mut raw)
        .map_err(|err| format!("cannot read stdin: {err}"))?;

    let outcome = fixer.fix(&raw);
    if let Some(value) = outcome.value() {
        println!("{value}");
    }
    for fix in outcome.fixes() {
        eprintln!("fixed {fix}");
    }
    if let Some(hint) = outcome.retry_hint() {
        eprintln!("{hint}");
    }
    Ok(match outcome {
        Outcome::Valid { .. } => EXIT_VALID,
        Outcome::Fixed { .. } => EXIT_FIXED,
        Outcome::Invalid { .. } => EXIT_INVALID,
    })
}

/// `Ok(None)` means help was requested.
fn parse_args(mut argv: impl Iterator<Item = String>) -> Result<Option<Args>, String> {
    let mut schema_path = None;
    let mut opts = FixOptions::default();
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(None),
            "--schema" => schema_path = Some(argv.next().ok_or("--schema needs a path")?),
            "--drop-additional" => opts.drop_additional_properties = true,
            "--no-defaults" => opts.fill_defaults = false,
            "--max-input-bytes" => {
                let text = argv.next().ok_or("--max-input-bytes needs a number")?;
                let limit = text
                    .parse()
                    .map_err(|_| format!("--max-input-bytes: not a number: {text}"))?;
                opts.max_input_bytes = Some(limit);
            }
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    let schema_path = schema_path.ok_or_else(|| format!("--schema is required\n{USAGE}"))?;
    Ok(Some(Args { schema_path, opts }))
}
