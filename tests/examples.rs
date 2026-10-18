//! Every example under `examples/` runs and prints what it should.

use assert_cmd::Command;

fn run(example: &str) -> String {
    let output = Command::cargo_bin("lbd")
        .unwrap()
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["run", &format!("examples/{example}")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{example} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn hello_world() {
    assert_eq!(run("hello_world.lbd"), "Hello, World\n");
}

#[test]
fn conditional_branching() {
    assert_eq!(
        run("conditional_branching.lbd"),
        "negative\nzero\npositive\ntrue\nfalse\n"
    );
}

#[test]
fn shapes() {
    assert_eq!(
        run("shapes.lbd"),
        "Areas: [3.14, 6.0]\nVolume of cylinder is: 785.0\n"
    );
}

#[test]
fn math_demos() {
    assert_eq!(
        run("math_demos.lbd"),
        "fibonacci 10 = 55\nfactorial 10 = 3628800\ngcd 1071 462 = 21\n\
         primes below 30: [2, 3, 5, 7, 11, 13, 17, 19, 23, 29]\n"
    );
}

#[test]
fn pair() {
    assert_eq!(run("pair.lbd"), "1\n2\n3\n");
}

#[test]
fn list() {
    assert_eq!(
        run("list.lbd"),
        "list: []\nlist: [\"Hello\"]\nlist size: 1\nlist: [\"Hello\", \"World\"]\nlist[1]: World\n\
         list: [\"Hello\", \"World\", \"Foo\"]\nwithout index 2: [\"Hello\", \"World\"]\n\
         unchanged: [\"Hello\", \"World\", \"Foo\"]\n"
    );
}

#[test]
fn std_library_type_checks() {
    assert_eq!(run("std.lbd"), "");
}

#[test]
fn advent_of_code_2024_day_1() {
    assert_eq!(run("aoc-24/day-01/part_01.lbd"), "Answer: 1223326\n");
    assert_eq!(run("aoc-24/day-01/part_02.lbd"), "Answer: 21070419\n");
}
