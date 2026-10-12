# Security policy

## Supported versions

Security fixes go into the latest release and the `main` branch.

| Version | Supported |
| --- | --- |
| Latest release | Yes |
| Older releases | No |

## Reporting a vulnerability

Please do not open a public issue. Report it privately through GitHub:
**Security → Report a vulnerability** on the repository
([direct link](https://github.com/lambda-discipline/lambda-discipline/security/advisories/new)).

Include the version (`lbd --version`), your platform, and a program or steps
that reproduce the problem. You should get an acknowledgement within a week.
We will agree on a disclosure date with you once a fix is ready, and credit
you in the advisory unless you prefer otherwise.

## Scope

Lambda Discipline runs programs with your user's permissions: `readFile`
reads any file you can read, and a program can use as much memory and time as
it likes (deep recursion stops at a configurable limit). Running an untrusted
`.lbd` program is like running an untrusted script. Things that are in scope:

* Crashes, hangs or memory exhaustion in the compiler itself (lexing,
  parsing, type checking) caused by a source file.
* A program behaving in a way the language does not allow, for example
  reaching the file system other than through `readFile`.
* Vulnerabilities in dependencies that affect `lbd`. Dependencies are checked
  in CI with `cargo deny` against the RustSec advisory database.
