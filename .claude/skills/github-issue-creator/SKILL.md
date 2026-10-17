---
name: github-issue-creator
description: Draft and create GitHub issues for Lambda Discipline (the lbd language) and its related repositories, following this project's templates, lowercase labels, issue linking, reproducible lbd output and repository routing. Use when asked to report a bug, propose a feature, turn review findings into issues, or file issues in the lambda-discipline organization.
---

# GitHub issues for Lambda Discipline

This skill extends the general issue-creation workflow with the conventions
of the `lambda-discipline` organization. Follow every rule below for every
issue, whether it is a single report or a batch.

## Workflow

1. Confirm `gh auth status` and the target repositories (see "Choosing the
   repository").
2. Read the target repository's issue templates, `CONTRIBUTING.md` and any
   `CLAUDE.md`. In the core repository these are
   `.github/ISSUE_TEMPLATE/bug_report.yml` and `feature_request.yml`.
3. List open and recently closed issues and pull requests in every target
   repository and search them for duplicates:
   `gh issue list -R <repo> --state all --limit 200 --search "<keywords>"`.
4. Gather evidence: read the relevant code and reproduce behaviour with `lbd`
   (see "Code samples and command output").
5. Draft each issue in the template's exact section structure.
6. Choose labels, creating missing ones in lowercase (see "Labels").
7. Create the issues, then link related ones (see "Linking issues").
8. Verify every created issue (title, labels, links, rendered body) and
   report each URL and number.

When the user asks for issues to be created, that is authorization to
publish the batch. Otherwise, present the drafts and ask before publishing.
Never invent bugs, output, versions or test results.

## Choosing the repository

File each issue where the change will be made. The core repository is not
the default for everything. Discover the organization's repositories instead
of relying on a fixed list, and skip forks and archived repositories:

```bash
gh repo list lambda-discipline --source --no-archived --json name,description
```

At the time of writing these are:

| Repository | File issues about |
| --- | --- |
| `lambda-discipline/lambda-discipline` | The Rust compiler and `lbd` binary: language, type system, builtins, interpreter, CLI, REPL, diagnostics, tests, CI, crate docs. |
| `lambda-discipline/lbd-emacs-major-mode` | Emacs editing: highlighting, indentation, keymaps in `lbd-mode.el`. |
| `lambda-discipline/lbd-emacs-inf-repl-mode` | Running the REPL inside Emacs (`lbd-inf-repl.el`, comint). |
| `lambda-discipline/lbd-vscode` | The VS Code extension. |
| `lambda-discipline/lbd-tm-grammar` | The TextMate grammar shared by editors. |
| `lambda-discipline/lbd-jupyter-kernel` | The Jupyter kernel. |
| `lambda-discipline/lambda-discipline.github.io` | The website, published reference pages and install instructions. |

Rules:

* A change that touches several repositories gets one issue per repository,
  each describing its own part, linked to each other (for example, removing
  a CLI flag in the core and updating the editor integration that passes it).
* Any repository in the organization can receive issues, including ones
  added after this skill was written. Read its README and source before
  filing there.
* Never file in archived repositories (`lambda-discipline-cpp`,
  `lambda-discipline-rust-legacy`) or in forks of third-party projects.
* Repositories without issue templates get the core feature or bug layout.

## Titles

* Format: `prefix: message`, matching the commit convention.
* Prefixes: `feat`, `fix`, `docs`, `test`, `ci`, `build`, `refactor`,
  `perf`, `chore`, `deps`.
* The message starts with a lowercase letter unless it starts with an
  abbreviation or a proper name (`Char`, `REPL`, `JSON`).
* No backticks anywhere in a title. Write names plainly
  (`fix: make range lazy instead of capping it at 1e8 elements`).

## Issue bodies

Use the template sections, in order, with nothing renamed:

* **Bug report:** Program, Command, Expected behavior, Actual behavior,
  Version, Operating system and Rust version, Anything else.
* **Feature or language proposal:** Area, Motivation, Proposal,
  Alternatives and prior art. Area is one of the template's dropdown values:
  `Language (syntax or semantics)`, `Type system`, `Builtins`,
  `REPL or CLI`, `Diagnostics`, `Backends and performance`,
  `Tooling, docs or other`.

Use the feature layout for tests, docs, CI, refactoring and tooling work
too, with the matching Area. Point at code with paths and item names
(`src/types/infer.rs`, `Checker::report`), state the concrete change, and
mention what tests or docs should change with it.

## Code samples and command output

Add a program, a proposed syntax example or real command output wherever it
helps: always for bugs, and for proposals whenever current behaviour can be
shown.

Every block of real output must state the `lbd` version it came from:

```bash
lbd --version                   # for example: lbd 0.1.0
git rev-parse --short HEAD      # the commit the binary was built from
rustc --version
```

Write it as, for example, `lbd 0.1.0 (commit c985d12)`, and put it in the
Version field of bug reports. In feature issues, write
"Observed with `lbd 0.1.0 (commit c985d12)` on <OS>:" above the output.

Capturing output:

* Build the binary from the commit you report (`cargo build`), and run it in
  a scratch directory on a file named `main.lbd`, so diagnostics show
  `main.lbd:1:13` rather than an absolute path. Remove any absolute or
  personal paths that remain.
* Set `NO_COLOR=1` so ariadne output has no escape codes.
* Show the command, the complete output and the exit code:

  ```text
  $ lbd run main.lbd
  Error: mismatched types
     ╭─[ main.lbd:1:13 ]
     ...
  [exit code 1]
  ```

* Keep reproductions minimal, and trim trailing whitespace from ariadne
  output.
* Put programs in fenced blocks tagged `haskell` (the template's `render`
  setting), output in `text` blocks, and shell commands in `bash` blocks.
* Label code that was not run, such as proposed syntax, as a proposal, and
  never present it as output.

## Labels

Label every issue. Check what exists first:

```bash
gh label list -R <repo> --limit 200
```

Core repository labels:

| Kind | Labels |
| --- | --- |
| Type | `bug`, `enhancement`, `documentation`, `refactoring`, `performance`, `testing` |
| Area | `language`, `type-system`, `builtins`, `interpreter`, `cli`, `repl`, `diagnostics`, `api`, `tooling`, `build`, `ci`, `dependencies` |
| Triage | `needs-design`, `good first issue`, `help wanted` |

Rules:

* Every issue gets one type label and at least one area label. Bug reports
  always get `bug`, and proposals get `enhancement` unless a more specific
  type label fits better.
* Add `needs-design` when the issue needs a design decision before
  implementation, and `good first issue` when the change is small and well
  described.
* If no existing label fits, create one with a lowercase, hyphenated name, a
  colour and a one-line description:

  ```bash
  gh label create type-system -R <repo> --color 5319e7 --description "Type checking and inference"
  ```

* Create the label in each repository that needs it, and reuse names across
  repositories where the meaning is the same.
* Never rename or delete existing labels.

## Linking issues

Link every issue to the issues it relates to, in the same repository and
across repositories:

* In the body, add a line like `**Related:** #5, #44` (for features, at the
  end of the Proposal section; for bugs, in "Anything else"). Say how they
  relate: "depends on #5", "blocked by #7", "unblocks #40", "see #18".
  Reference other repositories as
  `lambda-discipline/lbd-emacs-inf-repl-mode#1`.
* For hard dependencies, also set GitHub's native "blocked by" relationship.
  It works across repositories in the organization:

  ```bash
  blocker_id=$(gh api repos/lambda-discipline/<blocker-repo>/issues/<n> -q .id)
  gh api -X POST repos/lambda-discipline/<blocked-repo>/issues/<m>/dependencies/blocked_by -F issue_id=$blocker_id
  ```

* Link duplicates or follow-ups of existing issues instead of creating new
  ones.

When creating a batch, issue numbers are not known in advance. Write the
drafts with placeholders such as `{#3}`, create every issue first, then
substitute the real numbers and update the bodies with
`gh issue edit <n> -R <repo> --body-file <file>`. Check that no placeholder
is left anywhere.

## Model-assisted drafting disclosure

End every new issue body with a disclosure naming the model actually used.
Never guess a version or address. For example:

```markdown
---
*This issue was drafted with assistance from Claude Opus 5.5 (1M context) <noreply@anthropic.com>.*
```

## Creating and verifying

```bash
gh issue create -R <repo> --title "<prefix: message>" --body-file <file> --label <label> --label <label>
```

Write bodies to files and pass them with `--body-file`, so backticks and
code blocks survive the shell. After creating, check each issue with
`gh issue view <n> -R <repo>`: the title has no backticks, the labels are
right, links resolve, and code blocks render. Report every URL to the user.
