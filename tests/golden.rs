//! Golden tests: every `tests/golden/**/*.test.toml` manifest names a program
//! and what running it must produce:
//!
//! ```toml
//! file = "program.lbd"
//!
//! [expect]
//! exit_code = 0
//! stdout = "exact output\n"
//! stderr_contains = ["a fragment", "another"]
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    file: String,
    expect: Expect,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expect {
    exit_code: Option<i32>,
    stdout: Option<String>,
    stderr: Option<String>,
    #[serde(default)]
    stdout_contains: Vec<String>,
    #[serde(default)]
    stderr_contains: Vec<String>,
}

fn manifests(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            manifests(&path, found);
        } else if path.to_string_lossy().ends_with(".test.toml") {
            found.push(path);
        }
    }
}

fn check(manifest_path: &Path) -> Result<(), String> {
    let manifest: Manifest = toml::from_str(&fs::read_to_string(manifest_path).unwrap())
        .map_err(|e| format!("invalid manifest: {e}"))?;
    let program = manifest_path.parent().unwrap().join(&manifest.file);
    let output = Command::cargo_bin("lbd")
        .unwrap()
        .arg("run")
        .arg(&program)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let expect = &manifest.expect;
    let mut problems = Vec::new();
    if let Some(code) = expect
        .exit_code
        .filter(|&code| output.status.code() != Some(code))
    {
        problems.push(format!(
            "exit code {:?}, expected {code}",
            output.status.code()
        ));
    }
    if let Some(expected) = expect.stdout.as_ref().filter(|e| stdout != **e) {
        problems.push(format!(
            "stdout differs\n--- expected\n{expected}--- actual\n{stdout}"
        ));
    }
    if let Some(expected) = expect.stderr.as_ref().filter(|e| stderr != **e) {
        problems.push(format!(
            "stderr differs\n--- expected\n{expected}--- actual\n{stderr}"
        ));
    }
    for fragment in &expect.stdout_contains {
        if !stdout.contains(fragment.as_str()) {
            problems.push(format!("stdout lacks {fragment:?}"));
        }
    }
    for fragment in &expect.stderr_contains {
        if !stderr.contains(fragment.as_str()) {
            problems.push(format!("stderr lacks {fragment:?}"));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!("{}\n--- stderr\n{stderr}", problems.join("\n")))
    }
}

fn run_suite(dir: &str) {
    let mut found = Vec::new();
    manifests(&Path::new(env!("CARGO_MANIFEST_DIR")).join(dir), &mut found);
    found.sort();
    assert!(!found.is_empty(), "no manifests under {dir}");
    let failures: Vec<String> = found
        .iter()
        .filter_map(|path| {
            check(path)
                .err()
                .map(|e| format!("=== {}\n{e}", path.display()))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} failed:\n{}",
        failures.len(),
        found.len(),
        failures.join("\n")
    );
}

#[test]
fn programs() {
    run_suite("tests/golden/programs");
}

#[test]
fn diagnostics() {
    run_suite("tests/golden/diagnostics");
}
