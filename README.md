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
fibonacci : Int -> Int = \n.
    (if (lt n 2) n (add (fibonacci (sub n 1)) (fibonacci (sub n 2))))

compose = \f. \g. \x. (f (g x))          -- inferred: (a -> b) -> (c -> a) -> c -> b

type Shape = Circle Float | Rect Float Float

area : Shape -> Float = \s. match s
    | Circle r -> (mul 3.14 (mul r r))
    | Rect w h -> (mul w h)

(println (map fibonacci (range 0 10)))   -- [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
(println ((compose show (mul 2)) 21))   -- 42
(println (map area [(Circle 1.0), (Rect 2.0 3.0)]))   -- [3.14, 6.0]
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
| `:info <name>` | Type and definition site of a name, a type's declaration, or a builtin's docs. |
| `:load <file>` / `:reload` | Load and run a file / load the last one again. |
| `:env` (`:browse`) | Your definitions and their types. |
| `:docs [name]` | Builtin reference. |
| `:reset` | Forget every definition. |

Diagnostics are rendered with [ariadne](https://crates.io/crates/ariadne):

```text
Error: mismatched types
   ╭─[ examples/demo.lbd:1:17 ]
   │
 1 │ (println (add 1 1.5))
   │                 ─┬─
   │                  ╰─── expected `Int`, found `Float`
   │
   │ Help: `Int` and `Float` are not converted implicitly: write a float literal such as `1.0`, or convert with `(toFloat n)` or `(floor x)`
   │
   │ Note: in argument 2 of `add`
───╯
```

## The language

### Syntax

```text
item  = "use" STRING                         include another file (path relative to this file)
      | "type" UPPER ["<" LOWER, ... ">"] "=" ["|"] UPPER type* ("|" UPPER type*)*
                                             data type; see "Data types"
      | LOWER [":" scheme] "=" expr          definition; the annotation is optional
      | expr                                 an action, performed in order when the program runs
expr  = "\" LOWER [":" type] "." expr        lambda (also `λ`)
      | "let" LOWER [":" scheme] "=" expr "in" expr
      | "let" LOWER [":" type] "<-" expr "in" expr   perform an action and name its result
      | "match" expr ("|" pattern "->" expr)+
      | "(" expr expr+ ")"                   application; any expression can be called
      | "(" expr ")" | "(" ")"               grouping, unit
      | "[" expr, ... "]"                    list
      | INT | FLOAT | STRING | LOWER | UPPER UPPER is a constructor
pattern = UPPER pattern* | "_" | LOWER | INT | FLOAT | STRING | "(" ")" | "(" pattern ")"
type  = Int | Float | String | Bool | Unit | List<type> | IO<type> | Declared<type, ...>
      | lowercase-variable | type -> type | (type)
scheme = [context "=>"] type                 see "Type classes"
context = CLASS "<" LOWER ">" | "(" CLASS "<" LOWER ">", ... ")"
```

Names that start with a lowercase letter (or `_`) are values; names that
start with an uppercase letter are types and constructors.

* `42` and `-7` are `Int` literals; `3.14`, `1.0` and `1e-3` are `Float`
  literals (see "Numbers").
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
* `print` and `println` take any number of arguments of any types in `Show`
  and must be called directly. They return `IO<Unit>`.

### Data types

`type` declares a data type: a set of alternatives, each a constructor with
fields. `match` takes a value apart by trying patterns from top to bottom.

```haskell
type Option<a> = None | Some a

type Tree<a> =
    | Leaf
    | Node Tree<a> a Tree<a>

withDefault : a -> Option<a> -> a = \fallback. \o. match o
    | Some x -> x
    | None -> fallback

depth : Tree<a> -> Int = \t. match t
    | Leaf -> 0
    | Node left _ right -> (add 1 (max (depth left) (depth right)))

(println (map Some [1, 2]))              -- [(Some 1), (Some 2)]
(println (withDefault 0 None))           -- 0
```

* Type parameters are written in angle brackets, as in `List<a>`. Types may
  be recursive, mutually recursive and declared in any order.
* Constructors are curried functions: `Some : a -> Option<a>`, so
  `(map Some xs)` and `(Node Leaf)` work. A constructor without fields is a
  value. A declaration ends at the next line that starts in the first
  column.
* Patterns are constructors with patterns for every field, variables, `_`,
  number and string literals, and `()`. A variable may appear once per
  pattern.
* Every `match` must cover all values of its type; otherwise it is an error
  that names a missing case (`` `Rect _ _` is not covered ``). Numbers and
  strings need a `_` or variable arm. An `Int` pattern matches an `Int` and
  a `Float` pattern a `Float`.
* An arm's body extends as far as it can, like a `let` body, so a `match`
  inside the last arm of another needs parentheses.
* Fields are lazy, like list elements, and `match` evaluates only what it
  needs to choose an arm: `(match (Some (error "x")) | Some _ -> 1 | None -> 0)`
  is `1`.
* `eq`, `lt`, `compare`, `sort` and `show` work on data structurally (see
  "Type classes"). Values are ordered by constructor, in declaration order,
  then by their fields. `show` and `print` write them as code:
  `(Some (Node Leaf 1 Leaf))`.
* `Bool` is the built-in data type `type Bool = False | True`, so booleans
  can be matched too.
* In the REPL, declaring a type again makes a new type; values of the old one
  keep the old type.

### Numbers

There are two number types, and nothing converts between them implicitly:

* `Int` is a 64-bit integer. Arithmetic that overflows is a runtime error
  ("integer overflow: `(mul 9223372036854775807 2)` does not fit in an
  `Int`"), never a wrong answer, and an integer literal too large for 64 bits
  is a compile error.
* `Float` is a 64-bit IEEE float.
* `add sub mul neg abs pow` work on both (they are in the `Num` class, see
  "Type classes"), so `square = \x. (mul x x)` is
  `Num<a> => a -> a`. `(add 1 1.5)` is a type error: write `1.0`, or convert
  with `toFloat : Int -> Float` and `floor`, `ceil`, `round : Float -> Int`.
* `div` and `mod` are integer division, rounding toward negative infinity as
  in Haskell (`(div -7 2)` is `-4`, `(mod -7 2)` is `1`); dividing by zero is
  an error. `fdiv` divides floats, and `(fdiv 1.0 0.0)` is `Infinity`.
* Indices and counts are `Int`: `length`, `get`, `take`, `drop`, `remove`,
  `range`, `stringLength`, `compare` and `__LINE__`. `sum` adds up a
  `List<Int>`; for floats write `(foldl add 0.0 xs)`.
* An `Int` prints as its digits. A `Float` prints with the shortest digits
  that read back as the same number, as JavaScript prints it, but a whole
  float keeps its `.0` so it reads back as a `Float`: `42.0`, `3.14`,
  `0.30000000000000004`, `0.000001`, and an exponent below `1e-6` or from
  `1e21` up (`1e-7`, `1e+300`). `-0.0` prints as `0.0`.

### Type classes

Comparing and showing work on every value except functions and `IO` actions,
and arithmetic works on `Int` and `Float`. Four built-in classes say which
types support what, so a mistake such as `(eq \x. x \y. y)` is a type error
rather than a runtime error:

| Class | Builtins | Types in it |
| --- | --- | --- |
| `Eq` | `eq neq` | `Int`, `Float`, `String`, `Unit`, `Bool`, and lists and data types whose elements and fields are in it |
| `Ord` | `lt le gt ge compare sort min max` | the same as `Eq`; `Ord<a>` implies `Eq<a>` |
| `Num` | `add sub mul neg abs pow` | `Int` and `Float` |
| `Show` | `show`, and the arguments of `print` and `println` | the same as `Eq` |

```haskell
member : Eq<a> => a -> List<a> -> Bool =
    \x. \xs. (foldr \y. \found. (or (eq x y) found) False xs)

largest = \xs. (foldl max (head xs) xs)   -- inferred: Ord<a> => List<a> -> a

(println (sort [(Some 3), None, (Some 1)]))   -- [None, (Some 1), (Some 3)]
```

* Types get these classes automatically: there is no `deriving` clause, and
  a data type is in a class when all its fields are. `Option<a>` is in `Eq`
  when `a` is; `type Handler = Handler (String -> IO<Unit>)` is in none.
* Contexts are inferred, like the rest of a type, and `:type` shows them.
  An annotation must list the classes its type variables need, written in
  front of the type: `Eq<a> => ...`, or `(Ord<k>, Show<v>) => ...` for
  several.
* The classes only restrict which types are accepted; the builtins work
  structurally at run time, so there are no defaulting rules and a constraint
  on a type that is never fixed is harmless: `(show [])` is `"[]"`.
* There are no user-defined classes or instances yet.
* The REPL shows any value, functions included (`<function>`), because
  displaying a result is not the same as `show`.

### Evaluation

Call-by-need: arguments are evaluated only when used, and at most once. `if`,
`and` and `or` only evaluate what they need, so `(if c x (error "..."))` is
safe. Tail calls run in constant space and deep recursion is only limited by
memory (one million nested calls take about half a gigabyte; runaway
recursion stops with a "recursion is too deep" error instead of crashing).
A value that depends on itself (`x = (add x 1)`) is reported, not looped on.

Lists are immutable persistent vectors: `push`, `remove`, `get` and friends
return new lists in `O(log n)` and never change their argument. A list's
length is fixed when it is built, but its elements are lazy like arguments:
`(head [1, (error "boom")])` is `1`, `map` and `zipWith` compute an element
only when it is used, and `foldr` stops as soon as its function ignores the
rest of the list. Comparing, showing, printing or summing a list evaluates
the elements it inspects. Lists are always finite.

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

A relative path given to `readFile` is relative to the working directory, as
in other languages, so paths typed on the command line mean what the user
expects. `use` paths are different: they are relative to the file that
contains the `use`. To read a file that ships with the program, start its path
with `__DIR__`, the directory of the file it is written in (`.` in the REPL):

```haskell
let input <- (readFile (concat __DIR__ "/input.txt")) in   -- works from any directory
(println (length (lines input)))
```

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
| Numbers | `add sub mul neg abs pow div mod fdiv sqrt toFloat floor ceil round min max` |
| Comparison | `eq neq lt le gt ge compare` (structural, for types in `Eq` and `Ord`; see "Type classes") |
| Booleans | `not and or if`, and the constructors `True` and `False` |
| Strings | `concat show parseInt parseFloat stringLength chars split lines join trim` |
| Lists | `length isEmpty head tail get cons push remove append map filter foldl foldr reverse sort range zipWith transpose sum take drop` |
| Input/output | `pure bind then sequence print println readFile error` |
| Source location | `__FILE__ __DIR__ __LINE__` (spelled as in C and Elixir, since their value depends on where they are written) |

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
resolve              names ─▶ locals / globals / constructors / builtins ─▶ hir
  ▼
types::infer         Hindley-Milner, SCC-ordered, level-based generalization;
                     match exhaustiveness (types::exhaustive)
  ▼
core::lower          closure conversion + uncurrying + match compilation ─▶ core (Core IR)
  ▼
backend::Backend     today: backend::interp, a lazy abstract machine
```

| Module | Responsibility |
| --- | --- |
| `source`, `diagnostics` | File registry, byte spans, `Diagnostic` values rendered with ariadne. No stage prints or exits. |
| `syntax` | `logos` lexer and a hand-written parser (like rustc's) that reports several errors per run. |
| `loader`, `resolve`, `hir` | `use` expansion, scoping, "did you mean" suggestions, the resolved IR. |
| `types` | Type representation, the type environment (built-in and declared types and constructors, identified by id), unification with occurs check, rigid annotation variables, schemes, and Maranget's exhaustiveness check. |
| `builtins` | One `Builtin` enum declaring name, signature, module and docs, like GHC's primops. The type checker reads the signatures; each backend implements the enum. |
| `core` | The backend-facing IR: frame slots instead of names, explicit captures, multi-argument lambdas, saturated constructors, and flat `case` decision trees compiled from nested patterns. |
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
