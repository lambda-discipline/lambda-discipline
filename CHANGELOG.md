# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Lexer (`logos`) and recursive-descent parser with error recovery.
- `use` includes, resolved relative to the including file.
- Name resolution with "a similar name exists" suggestions.
- Hindley-Milner type inference with let-polymorphism, checked annotations
  and mutually recursive definitions.
- Core IR with closure conversion and uncurrying, behind a `Backend` trait.
- Lazy (call-by-need) interpreter as an explicit-stack machine: tail calls in
  constant space, deep recursion bounded by memory.
- Builtin library for numbers, comparison, booleans, strings, immutable lists
  and I/O, documented by `lbd docs`.
- `lbd run`, `lbd check [--emit tokens|ast|types|core]`, `lbd repl`, `lbd docs [--json]`.
- REPL with GHCi-style commands (`:type`, `:info`, `:load`, `:reload`, `:env`,
  `:docs`, `:reset`), multi-line input and tab completion.
- Diagnostics rendered with `ariadne`.

### Changed

- I/O is typed: `print`, `println` and `readFile` return `IO` actions, which
  run only when performed. `lbd run` performs the top-level expressions in
  source order, and a top-level expression that is not an action is a type
  error. `let x <- action in body` performs an action and names its
  result, and the new `sequence`, `then`, `bind` and `pure` builtins combine
  actions. Effects no longer depend on evaluation order (#8).

### Fixed

- Deeply nested programs no longer crash with a stack overflow: expressions
  and types nested more than 10,000 levels are reported as an error, and
  showing, comparing and freeing deeply nested values no longer uses the
  native stack (#31).
