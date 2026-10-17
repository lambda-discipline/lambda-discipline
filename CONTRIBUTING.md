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
cargo clippy --all-targets -- -D warnings
```

```bash
cargo test
```

```bash
cargo deny check
```

```bash
cargo +1.85 test
```

Keep pull requests focused, describe the user-visible change, and add tests:
unit tests next to the code you changed, plus a golden test for anything a
user can observe. Add a line to [CHANGELOG.md](CHANGELOG.md) under
"Unreleased" for user-facing changes.

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
