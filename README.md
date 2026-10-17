# Lambda Discipline

[![CI](https://github.com/lambda-discipline/lambda-discipline/actions/workflows/ci.yml/badge.svg)](https://github.com/lambda-discipline/lambda-discipline/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE.txt)
![MSRV 1.85](https://img.shields.io/badge/rustc-1.85+-orange.svg)

A lazy, statically typed functional programming language based on the
[lambda calculus](https://en.wikipedia.org/wiki/Lambda_calculus), written in
Rust. Programs are S-expression applications of `\x. body` lambdas, checked
with Hindley-Milner type inference and evaluated call-by-need, on a compiler
pipeline designed to grow more backends.

```haskell
-- Types are inferred; annotations are checked.
fibonacci : Number -> Number = \n.
    (if (lt n 2) n (add (fibonacci (sub n 1)) (fibonacci (sub n 2))))

compose = \f. \g. \x. (f (g x))          -- inferred: (a -> b) -> (c -> a) -> c -> b

(println (map fibonacci (range 0 10)))   -- [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
(println ((compose show (mul 2)) 21))   -- 42
```

## Getting started

Requires Rust 1.85 or newer.

```bash
cargo build --release
```

```bash
./target/release/lbd run examples/math_demos.lbd
```

```bash
./target/release/lbd repl
```

| Command | What it does |
| --- | --- |
| `lbd run <file>` | Type-check and run a program. Exit code 1 on any error. |
| `lbd check <file>` | Type-check only. |
| `lbd check <file> --emit tokens\|ast\|types\|core` | Print an intermediate representation. |
| `lbd repl` (or just `lbd`) | Interactive session. Without a terminal (pipes, Jupyter, editor modes) it reads stdin without prompts. `--repl` is accepted for the Emacs mode. |
| `lbd docs [name] [--json]` | Reference for the builtins (`--json` for tools). |

### REPL

Commands follow GHCi: they start with `:`, and any prefix works, with the
common commands winning ties (`:t` is `:type`, `:l` is `:load`, `:r` is
`:reload`). Tab completes names, commands and file paths after `:load`.

| Command | What it does |
| --- | --- |
| `:help` (`:?`) | List the commands. |
| `:quit` (`:exit`, Ctrl-D) | Leave. |
| `:type <expr>` | Type of an expression, without evaluating it. |
| `:info <name>` | Type and definition site of a name, or a builtin's docs. |
| `:load <file>` / `:reload` | Load and run a file / load the last one again. |
| `:env` (`:browse`) | Your definitions and their types. |
| `:docs [name]` | Builtin reference. |
| `:reset` | Forget every definition. |

Diagnostics are rendered with [ariadne](https://crates.io/crates/ariadne):

```text
Error: mismatched types
   ╭─[ examples/demo.lbd:1:13 ]
   │
 1 │ (print (add "hello" 5))
   │             ───┬───
   │                ╰───── expected `Number`, found `String`
   │
   │ Note: in argument 1 of `add`
───╯
```

## The language

### Syntax

```text
item  = "use" STRING                         include another file (path relative to this file)
      | IDENT [":" type] "=" expr            definition; the annotation is optional
      | expr                                 an action, performed in order when the program runs
expr  = "\" IDENT [":" type] "." expr        lambda (also `λ`)
      | "let" IDENT [":" type] "=" expr "in" expr
      | "let" IDENT [":" type] "<-" expr "in" expr   perform an action and name its result
      | "(" expr expr+ ")"                   application; any expression can be called
      | "(" expr ")" | "(" ")"               grouping, unit
      | "[" expr, ... "]"                    list
      | NUMBER | STRING | IDENT
type  = Number | String | Bool | Unit | List<type> | IO<type> | lowercase-variable | type -> type | (type)
```

* Numbers are 64-bit floats and print the way most languages print them: `42`,
  `3.14`, `0.30000000000000004`, `Infinity`.
* Strings support `\n \t \r \0 \\ \" \' \u{3bb}` escapes.
* Definitions are order independent and may be mutually recursive; `let` is
  recursive too.

### Types

Full Hindley-Milner inference with let-polymorphism:

* Unannotated definitions get their most general type (`id = \x. x` is `a -> a`).
* Annotations are checked, and their type variables are rigid:
  `f : a -> b = \x. x` is an error because the body is `a -> a`.
* Type variables in lambda annotations scope over the whole definition, so
  `apply : (a -> b) -> a -> b = \f: (a -> b). \x: a. (f x)` works.
* Mutually recursive definitions are inferred together, then generalized.
* `print` and `println` take any number of arguments of any types and must be
  called directly. They return `IO<Unit>`.

### Evaluation

Call-by-need: arguments are evaluated only when used, and at most once. `if`,
`and` and `or` only evaluate what they need, so `(if c x (error "..."))` is
safe. Tail calls run in constant space and deep recursion is only limited by
memory (one million nested calls take about half a gigabyte; runaway
recursion stops with a "recursion is too deep" error instead of crashing).
A value that depends on itself (`x = (add x 1)`) is reported, not looped on.

Lists are immutable persistent vectors: `push`, `remove`, `get` and friends
return new lists in `O(log n)` and never change their argument. Their elements
are evaluated when the list is.

### Input and output

Evaluating an expression never has an effect. `print`, `println` and
`readFile` return *actions* of type `IO<a>`: values that describe an effect
and produce an `a` when they are performed. `let x <- action in body` performs
an action and names its result, the way `let x = value in body` names a value:

```haskell
shout : String -> IO<Unit> = \path.
    let text <- (readFile path) in      -- perform an action, name its result
    let loud = (concat text "!") in     -- name a value, as always
    (println loud)                      -- the last action gives the result

(sequence [(println "a"), (shout "input.txt")])   -- perform a list of actions in order
```

`sequence` performs a list of actions and collects their results. `then`
performs one action after another, `bind` is the function behind `<-`, and
`pure` makes an action that just returns a value.

`lbd run` performs the program's top-level expressions in source order, and
nothing else: a definition such as `greeting = (println "hi")` only runs when a
top-level action uses it, and a top-level expression that is not an action is
a type error, because its value would be discarded. The REPL performs an
action it is given and shows its result; any other expression is evaluated
and shown. `error` is not an action: it stops the program when it is
evaluated, wherever that happens.

### Builtins

`lbd docs` prints the full reference. Every builtin is lowerCamelCase and
takes the data it works on last, so partial application reads well:
`(map (add 1) xs)`, `(filter (lt 0) xs)`, `(split "," line)`.

| Group | Builtins |
| --- | --- |
| Numbers | `add sub mul div mod pow neg abs floor ceil round sqrt min max` |
| Comparison | `eq neq lt le gt ge compare` (structural, like OCaml; comparing functions is a runtime error) |
| Booleans | `true false not and or if` |
| Strings | `concat show parseNumber stringLength chars split lines join trim` |
| Lists | `length isEmpty head tail get cons push remove append map filter foldl foldr reverse sort range zipWith transpose sum take drop` |
| Input/output | `pure bind then sequence print println readFile error` |

[`examples/std.lbd`](examples/std.lbd) is a small library written in the
language itself (`compose`, `flip`, Church pairs, `count`, `all`, `any`, `forEach`, ...).

## Architecture

The crate is a library (`src/lib.rs`) plus the `lbd` binary, organized as a
compiler pipeline. Each stage has its own module and its own unit tests.

```text
source text
  │  syntax::token   logos lexer
  │  syntax::parser  recursive descent with error recovery ─▶ syntax::ast
  ▼
loader               expands `use`, each file once
  ▼
resolve              names ─▶ locals / globals / builtins ─▶ hir
  ▼
types::infer         Hindley-Milner, SCC-ordered, level-based generalization
  ▼
core::lower          closure conversion + uncurrying ─▶ core (Core IR)
  ▼
backend::Backend     today: backend::interp, a lazy abstract machine
```

| Module | Responsibility |
| --- | --- |
| `source`, `diagnostics` | File registry, byte spans, `Diagnostic` values rendered with ariadne. No stage prints or exits. |
| `syntax` | `logos` lexer and a hand-written parser (like rustc's) that reports several errors per run. |
| `loader`, `resolve`, `hir` | `use` expansion, scoping, "did you mean" suggestions, the resolved IR. |
| `types` | Type representation, unification with occurs check, rigid annotation variables, schemes. |
| `builtins` | One `Builtin` enum declaring name, signature, module and docs, like GHC's primops. The type checker reads the signatures; each backend implements the enum. |
| `core` | The backend-facing IR: frame slots instead of names, explicit captures, multi-argument lambdas. |
| `backend::interp` | An STG-style machine with an explicit continuation stack, memoizing thunks, update frames, tail calls, and higher-order builtins as frames. |
| `session` | The driver; keeps globals across units so the REPL can feed one input at a time. |
| `repl`, `cli` | rustyline front end (with a line-mode fallback) and the clap command line. |

**Adding a builtin:** add a line to the `builtins!` table in `src/builtins.rs`
(name, signature, module, doc), then implement it in
`src/backend/interp/prims.rs` (first-order) or `src/backend/interp/mod.rs`
(builtins that control evaluation). The type checker and `lbd docs` pick it up
from the table.

**Adding a backend:** implement `backend::Backend` for a type whose `Output`
is whatever the backend produces (C or JavaScript source for a transpiler,
uxntal for uxn, an LLVM module), and feed it `Session::compile(...)?.module`.
The Core IR is already closure-converted and uncurried, which is the shape a
code generator wants; `tests/pipeline.rs` has a minimal example backend.

## Testing

```bash
cargo test
```

```bash
cargo clippy --all-targets -- -D warnings
```

* **Unit tests** live next to the code in every module (lexer, parser,
  loader, resolver, inference, lowering, values, primitives, machine, REPL,
  diagnostics).
* **Golden tests** (`tests/golden.rs`) run every `tests/golden/**/*.test.toml`
  manifest through the `lbd` binary and compare its exit code, stdout and
  stderr (see [CONTRIBUTING.md](CONTRIBUTING.md#golden-tests) for the format).
  `programs/` covers language features; `diagnostics/` checks error messages
  and locations.
* **Examples** (`tests/examples.rs`) run everything under `examples/`,
  including Advent of Code 2024 day 1.
* **CLI** (`tests/cli.rs`) covers subcommands, `--emit`, exit codes and the
  REPL over a pipe; **pipeline** (`tests/pipeline.rs`) covers the library API
  and plugging in another backend.

The minimum supported Rust version is 1.85; CI checks it, along with
formatting, clippy, tests on Linux, macOS and Windows, and `cargo deny`.

## Editor support

* [GNU Emacs major mode](https://github.com/lambda-discipline/lbd-emacs-major-mode)
* [GNU Emacs inferior REPL mode](https://github.com/lambda-discipline/lbd-emacs-inf-repl-mode)
* [VS Code extension](https://github.com/lambda-discipline/lbd-vscode)
* [Jupyter kernel](https://github.com/lambda-discipline/lbd-jupyter-kernel)

## Contributing

Bug reports, language proposals and pull requests are welcome; see
[CONTRIBUTING.md](CONTRIBUTING.md). Please follow the
[Code of Conduct](CODE_OF_CONDUCT.md), and report vulnerabilities as described
in [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE.txt)
