//! Command-line behaviour.

use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

fn lbd() -> Command {
    let mut command = Command::cargo_bin("lbd").unwrap();
    command.env("NO_COLOR", "1");
    command
}

fn program(text: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.lbd");
    fs::write(&path, text).unwrap();
    (dir, path)
}

#[test]
fn help_lists_subcommands() {
    lbd().arg("--help").assert().success().stdout(
        predicate::str::contains("run")
            .and(predicate::str::contains("check"))
            .and(predicate::str::contains("docs")),
    );
}

#[test]
fn version() {
    lbd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn run_prints_program_output() {
    let (_dir, path) = program("(println (add 1 2))");
    lbd()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("3\n")
        .stderr("");
}

#[test]
fn run_reports_type_errors_without_running() {
    let (_dir, path) = program("(println \"runs?\")\n(println (add 1 \"x\"))");
    lbd()
        .arg("run")
        .arg(&path)
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("mismatched types"));
}

#[test]
fn run_missing_file() {
    lbd()
        .args(["run", "does/not/exist.lbd"])
        .assert()
        .code(1)
        .stderr(predicate::str::starts_with(
            "error: cannot find `does/not/exist.lbd`",
        ));
}

#[test]
fn unknown_subcommand_is_a_usage_error() {
    lbd().arg("frobnicate").assert().code(2);
}

#[test]
fn check_succeeds_silently_or_reports() {
    let (_dir, path) = program("(println (error \"not run\"))");
    lbd()
        .arg("check")
        .arg(&path)
        .assert()
        .success()
        .stdout("")
        .stderr("");
    let (_dir, path) = program("x = (not 1)");
    lbd()
        .arg("check")
        .arg(&path)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("expected `Bool`, found `Number`"));
}

#[test]
fn emit_every_stage() {
    let (_dir, path) = program("twice = \\f. \\x. (f (f x))\n(println (twice (add 1) 0))");
    lbd()
        .args(["check", "--emit", "tokens"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("1:1 Ident \"twice\""));
    lbd()
        .args(["check", "--emit", "ast"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("twice = (\\f. (\\x. (f (f x))))"));
    lbd()
        .args(["check", "--emit", "types"])
        .arg(&path)
        .assert()
        .success()
        .stdout("twice : (a -> a) -> a -> a\n");
    lbd()
        .args(["check", "--emit", "core"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "@0 twice [frame=0] = (lambda/2 [] frame=2 ($0 ($0 $1)))",
        ));
}

#[test]
fn docs_lists_builtins() {
    lbd().arg("docs").assert().success().stdout(
        predicate::str::contains("map : (a -> b) -> List<a> -> List<b>")
            .and(predicate::str::contains("-- Strings")),
    );
    lbd()
        .args(["docs", "foldr"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with(
            "foldr : (a -> b -> b) -> b -> List<a> -> b\n",
        ));
    lbd().args(["docs", "nope"]).assert().code(1);
}

#[test]
fn docs_json_is_machine_readable() {
    lbd().args(["docs", "--json"]).assert().success().stdout(
        predicate::str::starts_with("{\n  \"version\": ")
            .and(predicate::str::contains("\"title\": \"Strings\"")),
    );
    lbd()
        .args(["docs", "--json", "foldr"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with(
            r#"{"name": "foldr", "signature": "(a -> b -> b) -> b -> List<a> -> b", "arity": 3"#,
        ));
}

#[test]
fn repl_reads_piped_input_without_prompts() {
    lbd()
        .arg("repl")
        .write_stdin(
            "double = \\x.\n  (mul x 2)\n(double 21)\n:type double\n(add 1 \"x\")\n(println \"after an error\")\n",
        )
        .assert()
        .success()
        .stdout("double : Number -> Number\n=> 42 : Number\ndouble : Number -> Number\nafter an error\n")
        .stderr(predicate::str::contains("mismatched types"));
}

#[test]
fn repl_is_the_default_and_accepts_the_legacy_flag() {
    lbd()
        .write_stdin("(add 2 2)\n")
        .assert()
        .success()
        .stdout("=> 4 : Number\n");
    lbd()
        .arg("--repl")
        .write_stdin(":quit\n(add 2 2)\n")
        .assert()
        .success()
        .stdout("");
}

#[test]
fn colors_are_disabled_when_not_a_terminal() {
    let (_dir, path) = program("(head [])");
    let output = Command::cargo_bin("lbd")
        .unwrap()
        .arg("run")
        .arg(&path)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`head` of an empty list"));
    assert!(!stderr.contains('\u{1b}'));
}

#[test]
fn deeply_nested_programs_are_reported_not_crashed_on() {
    let deep = 100_000;
    let (_dir, path) = program(&format!("{}1{}", "(".repeat(deep), ")".repeat(deep)));
    lbd()
        .arg("run")
        .arg(&path)
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(
            "expression nested too deeply (the limit is 10000 levels)",
        ));
}
