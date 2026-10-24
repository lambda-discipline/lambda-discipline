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
//!
//! Each manifest is its own test, so failures are reported one by one and a
//! filter such as `cargo test --test golden -- programs/fibonacci` runs one.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use assert_cmd::Command;
use libtest_mimic::{Arguments, Failed, Trial};
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

/// One test per manifest, named by its path under `tests/golden` without the
/// `.test.toml` suffix, for example `programs/fibonacci`.
fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut found = Vec::new();
    manifests(&root, &mut found);
    found.sort();
    assert!(!found.is_empty(), "no manifests under {}", root.display());
    let tests = found
        .into_iter()
        .map(|path| {
            let relative = path.strip_prefix(&root).unwrap().to_string_lossy();
            let name = relative
                .strip_suffix(".test.toml")
                .unwrap()
                .replace('\\', "/");
            Trial::test(name, move || check(&path).map_err(Failed::from))
        })
        .collect();
    libtest_mimic::run(&Arguments::from_args(), tests).exit_code()
}
