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

- Type errors no longer print an inferred type variable with the same name as
  an annotation's: checking `f : a -> Number = \x. (length x)` now reports
  that the definition has type `List<b> -> Number`, not `List<a> -> Number`
  (#16).
- Very large and very small numbers print with an exponent, as in
  JavaScript: `1e300` prints as `1e+300` instead of 301 digits, and `1e-7`
  as `1e-7` instead of `0.0000001`. Numbers from `1e-6` up to just below
  `1e21` still print positionally, `-0` prints as `0`, and `show`, `print`
  and `lbd check --emit ast|core` all use the same rules (#23).
- The infinite-type error always names the variable first: building
  `(cons acc [])` from `acc` now says "`a` would have to contain itself:
  `List<a>`" instead of the reversed "`List<a>` would have to contain
  itself: `a`" (#17).
- List elements are evaluated only when used, like arguments: `head`, `get`,
  `map`, `zipWith` and `foldr` no longer evaluate elements nobody needs, so
  `(foldr f z xs)` stops as soon as `f` ignores its second argument (#7).
- Deeply nested programs no longer crash with a stack overflow: expressions
  and types nested more than 10,000 levels are reported as an error, and
  showing, comparing and freeing deeply nested values no longer uses the
  native stack (#31).
