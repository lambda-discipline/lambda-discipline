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
- `lbd run`, `lbd check [--emit tokens|ast|types|core]`, `lbd repl`, `lbd docs`.
- REPL with GHCi-style commands (`:type`, `:info`, `:load`, `:reload`, `:env`,
  `:docs`, `:reset`), multi-line input and tab completion.
- Diagnostics rendered with `ariadne`.
