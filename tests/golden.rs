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
//!
//! `cargo test --test golden -- --bless` (or `BLESS=1` in the environment)
//! rewrites the exact expectations (`exit_code`, `stdout`, `stderr`) of every
//! selected manifest to what the program now produces, keeping the rest of
//! the file as written. A manifest whose `[expect]` table is empty gets its
//! `exit_code` and `stdout` filled in. Fragments (`*_contains`) are never
//! rewritten, so a missing fragment still fails.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Output};

use assert_cmd::Command;
use libtest_mimic::{Arguments, Failed, Trial};
use serde::Deserialize;
use toml_edit::{DocumentMut, Item, Value};

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

impl Expect {
    fn is_empty(&self) -> bool {
        self.exit_code.is_none()
            && self.stdout.is_none()
            && self.stderr.is_none()
            && self.stdout_contains.is_empty()
            && self.stderr_contains.is_empty()
    }
}

fn parse(text: &str) -> Result<Manifest, String> {
    toml::from_str(text).map_err(|e| format!("invalid manifest: {e}"))
}

fn check(manifest_path: &Path, bless: bool) -> Result<(), String> {
    let text = fs::read_to_string(manifest_path).unwrap();
    let mut manifest = parse(&text)?;
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
    if bless {
        let blessed = blessed(&text, &manifest.expect, &output, &stdout, &stderr);
        if blessed != text {
            fs::write(manifest_path, &blessed).unwrap();
            manifest = parse(&blessed)?;
        }
    }
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

/// The manifest `text` with the exact expectations that differ from what the
/// program produced replaced by it, so unchanged values keep their layout.
fn blessed(text: &str, expect: &Expect, output: &Output, stdout: &str, stderr: &str) -> String {
    let mut document: DocumentMut = text.parse().unwrap();
    let table = document["expect"].as_table_mut().unwrap();
    let fill = expect.is_empty();
    if let Some(code) = output
        .status
        .code()
        .filter(|&code| (fill || expect.exit_code.is_some()) && expect.exit_code != Some(code))
    {
        table["exit_code"] = Item::Value(i64::from(code).into());
    }
    if (fill || expect.stdout.is_some()) && expect.stdout.as_deref() != Some(stdout) {
        table["stdout"] = Item::Value(string(stdout));
    }
    if expect.stderr.is_some() && expect.stderr.as_deref() != Some(stderr) {
        table["stderr"] = Item::Value(string(stderr));
    }
    document.to_string()
}

/// A TOML string for `s`: a multi-line string when `s` is several whole
/// lines, as manifests write program output, and a one-line string otherwise.
fn string(s: &str) -> Value {
    if s.lines().count() < 2 || !s.ends_with('\n') {
        return s.into();
    }
    // The newline after the opening quotes is not part of the string.
    let mut repr = String::from("\"\"\"\n");
    let mut quotes = 0;
    for c in s.chars() {
        quotes = if c == '"' { quotes + 1 } else { 0 };
        match c {
            '\\' => repr.push_str("\\\\"),
            // A third quote in a row would close the string.
            '"' if quotes % 3 == 0 => repr.push_str("\\\""),
            '\n' | '\t' => repr.push(c),
            c if c.is_control() => write!(repr, "\\u{:04X}", u32::from(c)).unwrap(),
            c => repr.push(c),
        }
    }
    repr.push_str("\"\"\"");
    repr.parse().unwrap()
}

/// One test per manifest, named by its path under `tests/golden` without the
/// `.test.toml` suffix, for example `programs/fibonacci`.
fn main() -> ExitCode {
    let mut bless = std::env::var_os("BLESS").is_some_and(|v| !v.is_empty() && v != "0");
    let args = std::env::args().filter(|arg| {
        let flag = arg == "--bless";
        bless |= flag;
        !flag
    });
    let arguments = Arguments::from_iter(args.collect::<Vec<_>>());
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
            Trial::test(name, move || check(&path, bless).map_err(Failed::from))
        })
        .collect();
    libtest_mimic::run(&arguments, tests).exit_code()
}
