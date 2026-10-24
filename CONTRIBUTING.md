# Contributing to Lambda Discipline

Thanks for helping! Bug reports, language proposals, documentation, editor
integrations and code are all welcome. By taking part you agree to follow the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Reporting issues

Search existing issues first, then open one with the matching template:

* **Bugs:** include the smallest program that shows the problem, the command
  you ran, what you expected, what happened, `lbd --version` and your platform.
* **Features and language proposals:** explain the motivation, show example
  programs, and mention how other languages handle it.
* **Security problems:** do not open a public issue; see [SECURITY.md](SECURITY.md).

## Development setup

You need a Rust toolchain; `rust-toolchain.toml` selects stable with
`rustfmt` and `clippy`. The crate supports Rust 1.85 and newer. Building
`mimalloc` also needs a C compiler (any that `cc` can find).

```bash
cargo build
```

```bash
cargo run -- run examples/math_demos.lbd
```

Optional but recommended: install the [pre-commit](https://pre-commit.com)
hooks, which format, lint and (before a push) test your changes.

```bash
pre-commit install --hook-type pre-commit --hook-type pre-push
```

## Before you open a pull request

Everything CI checks can be run locally:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo test --workspace
```

```bash
cargo deny check
```

```bash
cargo +1.85 test --workspace
```

Keep pull requests focused, describe the user-visible change, and add tests:
unit tests next to the code you changed, plus a golden test for anything a
user can observe. Add a line to [CHANGELOG.md](CHANGELOG.md) under
"Unreleased" for user-facing changes.

Pull requests may be reviewed with Claude Code, which follows the project
skill in `.claude/skills/pr-review/SKILL.md`: it checks whether the change
is safe to run, runs the checks above, reviews commits, tests, the
changelog and language-design consistency, and posts its findings as a
comment on the pull request.

## Code layout

The compiler is a pipeline; each stage lives in its own module with its own
tests. [README.md](README.md#architecture) describes the stages.

* **Adding a builtin:** add a row to the `builtins!` table in
  `src/builtins.rs` (name, type signature, module, one-line doc). Implement it
  in `src/backend/interp/prims.rs` if it only needs its forced arguments, or
  in `src/backend/interp/mod.rs` if it controls evaluation (laziness, calling
  functions). The type checker and `lbd docs` read the table.
* **Adding a REPL command:** add an entry to `COMMANDS` in
  `src/repl/commands.rs` and handle it in `Repl::command`.
* **Adding a backend:** implement `backend::Backend` over the Core IR in
  `src/core`; see `tests/pipeline.rs` for a minimal example.
* **Diagnostics:** stages return `Diagnostic` values; they never print or
  exit. Point the span at the most specific location and phrase labels as
  what is wrong there.

## Golden tests

`tests/golden.rs` runs every `tests/golden/**/*.test.toml` manifest. A
manifest names a program next to it and what running `lbd run` on it must
produce:

```toml
file = "program.lbd"

[expect]
exit_code = 0                         # optional
stdout = """
exact output
"""                                   # optional, exact match
stderr = "exact error output"         # optional, exact match
stdout_contains = ["fragment"]        # optional
stderr_contains = ["fragment"]        # optional
```

Put programs that exercise language features in `tests/golden/programs/` and
programs that should fail, with their messages and `file:line:column`
locations, in `tests/golden/diagnostics/`. Golden files are compared byte for
byte, so keep them LF-terminated (`.gitattributes` does this for you).

Each manifest is its own test, named by its path without `.test.toml`, so you
can run one program, or every program whose name matches, on its own:

```sh
cargo test --test golden -- programs/fibonacci
cargo test --test golden -- diagnostics/
```

## Property tests

`tests/properties.rs` checks laws the compiler must obey for every program,
on random programs from `crates/lbd-smith`. That generator builds programs
that are well-typed by construction and records the type it meant each
definition to have. The laws: printing a parsed program and parsing it again
gives the same tree; number literals read back exactly; generated programs
type-check with and without annotations; inferred types written back as
annotations are accepted; renaming variables and reordering definitions
doesn't change any type; ill-typed mutants are rejected; Core is
well-scoped; `match` exhaustiveness agrees with brute-force enumeration;
and `eq` and `compare` obey the laws of equality and a total order.

They run with `cargo test` on every platform, for about 100 ms each. To
look harder, raise the budget:

```bash
ARBTEST_BUDGET_MS=60000 cargo test --release --test properties
```

A failure prints the program and a seed. `ARBTEST_SEED=<seed>` reruns
exactly that case. Once you've fixed the bug, add the program as a golden
test.

## Fuzzing

`fuzz/` holds [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html)
targets that feed arbitrary text to the compiler and check that it never
panics, hangs or reports a location outside the file:

* `lex`: the lexer. Tokens must tile the input in order.
* `parse`: the parser and the AST printer.
* `check`: every stage before evaluation, then diagnostic rendering. `use`
  reads from an empty in-memory file set, so inputs never touch the disk.
* `check_gen`: the input drives `lbd-smith`, so every program reaches the
  type checker. Well-typed programs must be accepted and their ill-typed
  mutants rejected.

CI runs each target for a minute on every pull request and for half an hour
each night, keeping the corpus between runs. Running them yourself needs
nightly Rust and Linux or macOS (on Windows, use WSL):

```bash
cargo install cargo-fuzz
```

```bash
mkdir -p fuzz/corpus/check
cargo +nightly fuzz run check fuzz/corpus/check examples tests/golden -- -dict=fuzz/lbd.dict
```

The first directory collects new inputs; the examples and golden programs
only seed it. When a target fails, libFuzzer saves the input under
`fuzz/artifacts/<target>/` (CI uploads it as an artifact). Shrink it with
`cargo +nightly fuzz tmin <target> <file>`, fix the bug, and add the shrunk
program as a golden test (in `tests/golden/diagnostics/` if it must be
rejected), so the fix is checked by `cargo test` on every platform.

## Commit messages

Write the subject as `prefix: message (#N)`, where `#N` is the issue the
commit addresses:

* Prefixes: `feat`, `fix`, `docs`, `test`, `ci`, `build`, `refactor`,
  `perf`, `chore`, `deps`.
* The message is imperative and starts with a lowercase letter, unless it
  starts with an abbreviation or a proper name (`REPL`, `JSON`). No trailing
  period and no backticks.
* Leave out `(#N)` only when there is no related issue.

Optionally, add a body after a blank line, wrapped at 72 columns, that
explains what changes and why. Don't repeat the issue reference
(`Closes #N`) in the body: the pull request description closes the issue.

If you used AI to write the change, end the message with a
`Co-Authored-By` trailer naming the model. This is required.

```text
fix: report deeply nested programs instead of overflowing the stack (#31)

The parser now rejects expressions and types nested more than 10,000
levels with a diagnostic, which bounds the recursion of every later
stage well within the front end's stack.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

Pull request and issue titles use the same `prefix: message` style.
Claude Code follows these rules through the project skill in
`.claude/skills/git-commit/SKILL.md`.
