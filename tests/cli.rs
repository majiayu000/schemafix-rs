use std::io::Write;
use std::process::{Command, Output, Stdio};

const SCHEMA: &str = r#"{"type":"object","properties":{"n":{"type":"integer"}},"required":["n"]}"#;

fn schema_file(name: &str, body: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("schemafix-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    path
}

fn cli(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_schemafix"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn run_schema(stdin: &str) -> Output {
    let path = schema_file("ok.json", SCHEMA);
    cli(&["--schema", path.to_str().unwrap()], stdin)
}

#[test]
fn exit_0_when_valid() {
    let out = run_schema(r#"{"n": 1}"#);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), r#"{"n":1}"#);
}

#[test]
fn exit_1_when_fixed() {
    let out = run_schema(r#"{"n": "1""#);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), r#"{"n":1}"#);
    assert!(String::from_utf8_lossy(&out.stderr).contains("string_to_number"));
}

#[test]
fn exit_2_when_invalid() {
    let out = run_schema(r#"{"n": "one"}"#);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("- /n: "));
}

#[test]
fn exit_3_on_bad_schema_or_usage() {
    let bad = schema_file("bad.json", r#"{"type": 12}"#);
    assert_eq!(
        cli(&["--schema", bad.to_str().unwrap()], "{}")
            .status
            .code(),
        Some(3)
    );
    assert_eq!(
        cli(&["--schema", "/nonexistent/s.json"], "{}")
            .status
            .code(),
        Some(3)
    );
    assert_eq!(cli(&[], "{}").status.code(), Some(3));
    assert_eq!(cli(&["--bogus"], "{}").status.code(), Some(3));
}
