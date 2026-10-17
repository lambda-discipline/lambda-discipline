---
name: git-commit
description: Write git commit messages for Lambda Discipline (the lbd language) in the project's format, with a prefix subject, the referenced issue number, an optional body and a Co-Authored-By trailer whenever AI assisted. Use whenever creating, drafting or rewording a commit in this repository.
---

# Git commits for Lambda Discipline

Every commit in this repository uses the same shape as the issue titles in
the `github-issue-creator` skill, so commits, issues and pull requests read
alike. Follow these rules for every commit.

## Message format

```text
prefix: message (#N)

Optional body explaining what changes for a user and why, wrapped at
72 columns.

Co-Authored-By: <model actually used> <address>
```

### Subject line

* Format: `prefix: message (#N)`, where `#N` is the issue the commit
  addresses.
* Prefixes: `feat`, `fix`, `docs`, `test`, `ci`, `build`, `refactor`,
  `perf`, `chore`, `deps`.
* The message starts with a lowercase letter unless it starts with an
  abbreviation or a proper name (`REPL`, `JSON`, `Char`).
* Use the imperative mood ("add", not "added" or "adds"), and no trailing
  period.
* No backticks in the subject. Write names plainly.
* When the commit resolves an issue, the subject usually matches the issue
  title, followed by its number.
* Include `(#N)` whenever a related issue exists. A commit with no issue
  (for example, a typo fix) omits it. Never invent a number.

### Body

* Optional. Separate it from the subject with a blank line and wrap it at
  72 columns.
* Explain what changes for someone using `lbd` and why, not a file-by-file
  list. Backticks are fine here.
* Do not repeat the issue reference (`Closes #8`, `Fixes #31`): it is
  already in the subject. Issues are closed by the pull request description.

### AI disclosure

* Any commit written with AI assistance must end with a `Co-Authored-By`
  trailer naming the model actually used. Never guess a version, context
  window or address. For example:

  ```text
  Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
  ```

* The trailer is the final block of the message, separated by a blank line,
  so `git interpret-trailers` and GitHub's co-author display pick it up.
* Do not add a separate disclosure paragraph. Commits written without AI
  omit the trailer.

## Examples

A commit with a body:

```text
fix: report deeply nested programs instead of overflowing the stack (#31)

The parser now rejects expressions and types nested more than 10,000
levels with a diagnostic, which bounds the recursion of every later
stage well within the front end's stack.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

A commit without a body:

```text
docs: fix the install command in the README (#70)

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
```

## Workflow

1. Read `CONTRIBUTING.md` and `git log --oneline -20` to confirm the
   convention.
2. Find the issue being worked on: from the branch name
   (`feat/8-io-effect-model` is #8), the task, or
   `gh issue list --search "<keywords>"`.
3. Stage only the files that belong to the change, and review
   `git diff --staged`. Never include unrelated changes.
4. Write the message to a file and commit with `git commit -F <file>`, so
   backticks and blank lines survive the shell.
5. Never skip hooks (`--no-verify`) or bypass signing. If a hook fails, fix
   the cause and create a new commit rather than amending one that is
   already pushed.
6. Check the result with `git log -1 --format=%B`.

## Pull requests

Pull request titles use the same `prefix: message` style. The pull request
description carries the closing keyword (`Closes #N`), since commits do not.
