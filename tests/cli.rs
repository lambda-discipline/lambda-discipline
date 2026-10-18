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
        .stderr(predicate::str::contains("expected `Bool`, found `Int`"));
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
fn emit_prints_numbers_like_run() {
    let (_dir, path) = program(
        "big = 1e300
small = 1e-7
(println big small)",
    );
    lbd()
        .args(["check", "--emit", "ast"])
        .arg(&path)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("big = 1e+300").and(predicate::str::contains("small = 1e-7")),
        );
    lbd()
        .args(["check", "--emit", "core"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("1e+300").and(predicate::str::contains("1e-7")));
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
        .stdout("double : Int -> Int\n=> 42 : Int\ndouble : Int -> Int\nafter an error\n")
        .stderr(predicate::str::contains("mismatched types"));
}

#[test]
fn repl_is_the_default_and_accepts_the_legacy_flag() {
    lbd()
        .write_stdin("(add 2 2)\n")
        .assert()
        .success()
        .stdout("=> 4 : Int\n");
    lbd()
        .arg("--repl")
        .write_stdin(":quit\n(add 2 2)\n")
        .assert()
        .success()
        .stdout("");
}

#[test]
fn colors_are_disabled_when_not_a_terminal() {
    let (_dir, path) = program("(unwrap (head []))");
    let output = Command::cargo_bin("lbd")
        .unwrap()
        .arg("run")
        .arg(&path)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`unwrap` of `None`"));
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

#[test]
fn read_file_is_relative_to_the_working_directory_and_dir_to_the_file() {
    let dir = tempfile::tempdir().unwrap();
    for sub in ["sub", "lib"] {
        fs::create_dir(dir.path().join(sub)).unwrap();
    }
    fs::write(dir.path().join("data.txt"), "working directory\n").unwrap();
    fs::write(dir.path().join("sub/data.txt"), "next to main\n").unwrap();
    fs::write(dir.path().join("lib/data.txt"), "next to lib\n").unwrap();
    fs::write(
        dir.path().join("lib/lib.lbd"),
        "libData : String = (concat __DIR__ \"/data.txt\")\n\
         libFile : String = __FILE__\n\
         libLine : Int = __LINE__\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("sub/main.lbd"),
        r#"use "../lib/lib.lbd"
read : String -> IO<Unit> = \path. let text <- (readFile path) in (print text)
(read "data.txt")
(read (concat __DIR__ "/data.txt"))
(read libData)
(println libFile)
(println "lib line " libLine ", main line " __LINE__)
"#,
    )
    .unwrap();
    let canonical = fs::canonicalize(dir.path().join("lib/lib.lbd")).unwrap();
    let shown = canonical.display().to_string();
    let lib_file = shown.strip_prefix(r"\\?\").unwrap_or(&shown);
    lbd()
        .current_dir(dir.path())
        .args(["run", "sub/main.lbd"])
        .assert()
        .success()
        .stdout(format!(
            "working directory\nnext to main\nnext to lib\n{lib_file}\nlib line 3, main line 7\n"
        ))
        .stderr("");
}
