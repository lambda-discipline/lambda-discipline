//! The interactive read-eval-print loop.
//!
//! [`Repl`] holds the logic and writes to injectable streams, so it is unit
//! tested without a terminal. [`run`] wires it to `rustyline` when stdin is a
//! terminal, and to plain line reading otherwise (pipes, editor integrations,
//! Jupyter), in which case no banner or prompts are printed.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::rc::Rc;

use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Editor, Helper};

use crate::builtins::Builtin;
use crate::session::Session;

mod commands;

use crate::syntax::token::{TokenKind, lex};
use commands::Command;

/// Whether the loop should keep going.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Continue,
    Quit,
}

pub struct Repl {
    session: Session,
    out: Box<dyn Write>,
    err: Box<dyn Write>,
    color: bool,
    /// Last byte the program printed, to keep `=>` lines on their own line.
    last_program_byte: Rc<Cell<Option<u8>>>,
    /// The file `:reload` loads again.
    last_loaded: Option<PathBuf>,
}

impl Repl {
    /// `program_out` receives what programs print; `out` receives REPL
    /// responses; `err` receives diagnostics.
    pub fn new(
        program_out: Box<dyn Write>,
        out: Box<dyn Write>,
        err: Box<dyn Write>,
        color: bool,
    ) -> Self {
        let last_program_byte = Rc::new(Cell::new(None));
        let tracked = Tracking {
            inner: program_out,
            last: last_program_byte.clone(),
        };
        Self {
            session: Session::new(Box::new(tracked)),
            out,
            err,
            color,
            last_program_byte,
            last_loaded: None,
        }
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Names to offer for tab completion.
    pub fn completion_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Builtin::ALL.iter().map(|b| b.name().to_owned()).collect();
        names.extend(self.session.bindings().iter().map(|g| g.name.to_string()));
        let types = self.session.types();
        names.extend(types.type_names().map(|n| n.to_string()));
        names.extend(types.constructor_names().map(|n| n.to_string()));
        names.sort();
        names.dedup();
        names
    }

    /// Handles one complete input: a command, a definition or an expression.
    pub fn handle(&mut self, input: &str) -> Control {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Control::Continue;
        }
        if let Some(command) = trimmed.strip_prefix(':') {
            return self.command(command);
        }
        self.last_program_byte.set(None);
        match self.session.eval("<repl>", input) {
            Ok(evaluated) => {
                self.session.flush();
                self.end_program_line();
                for (name, scheme) in &evaluated.definitions {
                    let _ = writeln!(self.out, "{name} : {scheme}");
                }
                for (value, scheme) in &evaluated.values {
                    if !matches!(value, crate::backend::interp::Value::Unit) {
                        let _ = writeln!(self.out, "=> {} : {}", value.show(), scheme);
                    }
                }
            }
            Err(diagnostics) => {
                self.session.flush();
                self.end_program_line();
                self.session.render(&diagnostics, &mut self.err, self.color);
            }
        }
        let _ = self.out.flush();
        let _ = self.err.flush();
        Control::Continue
    }

    /// Starts a new line if the program left the cursor mid-line.
    fn end_program_line(&mut self) {
        if self.last_program_byte.get().is_some_and(|b| b != b'\n') {
            let _ = writeln!(self.out);
        }
    }

    fn command(&mut self, input: &str) -> Control {
        let (typed, argument) = input
            .split_once(char::is_whitespace)
            .map_or((input, ""), |(name, rest)| (name, rest.trim()));
        let Some(command) = commands::lookup(typed) else {
            let _ = writeln!(
                self.err,
                "unknown command `:{typed}`; type :help for a list"
            );
            let _ = self.err.flush();
            return Control::Continue;
        };
        match command {
            Command::Quit => return Control::Quit,
            Command::Help => {
                let _ = writeln!(self.out, "{}", commands::help());
            }
            Command::Type if argument.is_empty() => self.usage(":type <expr>"),
            Command::Type => match self.session.type_of(argument) {
                Ok(scheme) => {
                    let _ = writeln!(self.out, "{argument} : {scheme}");
                }
                Err(diagnostics) => self.session.render(&diagnostics, &mut self.err, self.color),
            },
            Command::Info if argument.is_empty() => self.usage(":info <name>"),
            Command::Info => self.info(argument),
            Command::Load if argument.is_empty() => self.usage(":load <file>"),
            Command::Load => self.load(PathBuf::from(argument)),
            Command::Reload => match self.last_loaded.clone() {
                Some(path) => self.load(path),
                None => {
                    let _ = writeln!(self.err, "nothing to reload; use :load <file> first");
                }
            },
            Command::Env => {
                let bindings = self.session.bindings();
                if bindings.is_empty() {
                    let _ = writeln!(self.out, "no definitions yet");
                }
                for binding in bindings {
                    let _ = writeln!(self.out, "{} : {}", binding.name, binding.scheme);
                }
            }
            Command::Docs if argument.is_empty() => {
                let _ = write!(self.out, "{}", crate::builtins::reference());
            }
            Command::Docs => match Builtin::lookup(argument) {
                Some(builtin) => self.describe_builtin(builtin),
                None => {
                    let _ = writeln!(self.err, "no builtin named `{argument}`");
                }
            },
            Command::Reset => {
                self.session.reset();
                self.last_loaded = None;
                let _ = writeln!(self.out, "environment cleared");
            }
        }
        let _ = self.out.flush();
        let _ = self.err.flush();
        Control::Continue
    }

    fn usage(&mut self, usage: &str) {
        let _ = writeln!(self.err, "usage: {usage}");
    }

    fn load(&mut self, path: PathBuf) {
        self.last_program_byte.set(None);
        let result = self.session.load(&path);
        self.session.flush();
        self.end_program_line();
        match result {
            Ok(evaluated) => {
                let count = evaluated.definitions.len();
                let _ = writeln!(
                    self.out,
                    "loaded {} ({count} definition{})",
                    path.display(),
                    if count == 1 { "" } else { "s" }
                );
                self.last_loaded = Some(path);
            }
            Err(diagnostics) => self.session.render(&diagnostics, &mut self.err, self.color),
        }
    }

    /// `:info`: a definition's type and where it was defined, a builtin's
    /// docs, or a type's or constructor's declaration.
    fn info(&mut self, name: &str) {
        let types = self.session.types();
        let declared = if let Some(con) = types.lookup_constructor(name) {
            let info = types.constructor(con);
            Some((
                format!(
                    "{} : {}\n    a constructor of `{}`",
                    info.name,
                    info.scheme,
                    types.ty(info.ty).con.name
                ),
                info.span,
            ))
        } else {
            types
                .lookup_type(name)
                .map(|info| (types.describe(info.con.id), info.span))
        };
        if let Some((text, span)) = declared {
            let _ = writeln!(self.out, "{text}");
            if let Some(span) = span {
                let file = self.session.sources.get(span.file);
                let (line, column) = file.line_col(span.start as usize);
                let _ = writeln!(self.out, "    defined at {}:{line}:{column}", file.name());
            }
            return;
        }
        if let Some(binding) = self.session.binding(name) {
            let file = self.session.sources.get(binding.span.file);
            let (line, column) = file.line_col(binding.span.start as usize);
            let _ = writeln!(
                self.out,
                "{} : {}\n    defined at {}:{line}:{column}",
                binding.name,
                binding.scheme,
                file.name()
            );
        } else if let Some(builtin) = Builtin::lookup(name) {
            self.describe_builtin(builtin);
        } else {
            let _ = writeln!(self.err, "`{name}` is not defined");
        }
    }

    fn describe_builtin(&mut self, builtin: Builtin) {
        let _ = writeln!(
            self.out,
            "{} : {}\n    {}",
            builtin.name(),
            builtin.display_signature(),
            builtin.doc()
        );
    }
}

/// Whether `text` is a complete input, or the user is still typing.
pub fn is_complete(text: &str) -> bool {
    if text.trim_start().starts_with(':') {
        return true;
    }
    let mut sources = crate::source::SourceMap::new();
    let file = sources.add_virtual("<input>", text);
    let (tokens, errors) = lex(file, text);
    if errors
        .iter()
        .any(|e| e.message == "unterminated string literal")
    {
        return false;
    }
    let mut depth = 0i32;
    // Whether a `match` has no arm yet: its arms are still to come.
    let mut armless_match = false;
    for token in &tokens {
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket => depth += 1,
            TokenKind::RParen | TokenKind::RBracket => depth -= 1,
            TokenKind::Match => armless_match = true,
            TokenKind::Bar => armless_match = false,
            _ => {}
        }
    }
    if depth > 0 || armless_match {
        return false;
    }
    let last = tokens
        .iter()
        .rev()
        .find(|t| t.kind != TokenKind::Eof)
        .map(|t| t.kind);
    !matches!(
        last,
        Some(
            TokenKind::Lambda
                | TokenKind::Dot
                | TokenKind::Equals
                | TokenKind::Arrow
                | TokenKind::Colon
                | TokenKind::Comma
                | TokenKind::Let
                | TokenKind::In
                | TokenKind::Use
                | TokenKind::LAngle
                | TokenKind::LeftArrow
                | TokenKind::Type
                | TokenKind::Match
                | TokenKind::Bar
        )
    )
}

/// Runs the REPL on the process's standard streams.
pub fn run(color: bool) -> io::Result<()> {
    let interactive = io::stdin().is_terminal();
    let mut repl = Repl::new(
        Box::new(io::stdout()),
        Box::new(io::stdout()),
        Box::new(io::stderr()),
        color,
    );
    if interactive {
        run_interactive(&mut repl, color)
    } else {
        run_lines(&mut repl, io::stdin().lock())
    }
}

/// Reads complete inputs from any line source, without prompts.
pub fn run_lines(repl: &mut Repl, input: impl BufRead) -> io::Result<()> {
    let mut pending = String::new();
    for line in input.lines() {
        pending.push_str(&line?);
        pending.push('\n');
        if is_complete(&pending) {
            let control = repl.handle(&std::mem::take(&mut pending));
            if control == Control::Quit {
                return Ok(());
            }
        }
    }
    if !pending.trim().is_empty() {
        repl.handle(&pending);
    }
    Ok(())
}

fn run_interactive(repl: &mut Repl, color: bool) -> io::Result<()> {
    let names = Rc::new(RefCell::new(repl.completion_names()));
    let mut editor: Editor<ReplHelper, DefaultHistory> = Editor::new().map_err(io::Error::other)?;
    editor.set_helper(Some(ReplHelper {
        names: names.clone(),
        files: FilenameCompleter::new(),
        color,
    }));
    let history = history_path();
    if let Some(path) = &history {
        let _ = editor.load_history(path);
    }

    println!(
        "Lambda Discipline {} REPL. Type :help for help, :quit to exit.",
        env!("CARGO_PKG_VERSION")
    );
    // Colors are added by `highlight_prompt`, so rustyline measures the
    // prompt without escape codes and puts the cursor right after it.
    let prompt = "λ> ";
    loop {
        match editor.readline(prompt) {
            Ok(input) => {
                let _ = editor.add_history_entry(input.as_str());
                if repl.handle(&input) == Control::Quit {
                    break;
                }
                *names.borrow_mut() = repl.completion_names();
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(error) => return Err(io::Error::other(error)),
        }
    }
    if let Some(path) = &history {
        let _ = editor.save_history(path);
    }
    Ok(())
}

fn history_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".lbd_history"))
}

/// rustyline integration: multi-line input and name completion.
struct ReplHelper {
    names: Rc<RefCell<Vec<String>>>,
    files: FilenameCompleter,
    color: bool,
}

impl Helper for ReplHelper {}
impl Highlighter for ReplHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        if self.color {
            Cow::Owned(format!("[36m{prompt}[0m"))
        } else {
            Cow::Borrowed(prompt)
        }
    }
}

impl Hinter for ReplHelper {
    type Hint = String;
}

impl Validator for ReplHelper {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        Ok(if is_complete(ctx.input()) {
            ValidationResult::Valid(None)
        } else {
            ValidationResult::Incomplete
        })
    }
}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let before = &line[..pos];
        if let Some(command) = before.strip_prefix(':') {
            match command.split_once(char::is_whitespace) {
                // Still typing the command name.
                None => {
                    let names = commands::completions(command);
                    return Ok((0, names.into_iter().map(pair).collect()));
                }
                Some((typed, _)) if commands::lookup(typed) == Some(Command::Load) => {
                    return self.files.complete(line, pos, ctx);
                }
                Some(_) => {}
            }
        }
        let start = before
            .rfind(|c: char| !(c.is_alphanumeric() || c == '_' || c == '\''))
            .map_or(0, |i| {
                i + before[i..].chars().next().map_or(1, char::len_utf8)
            });
        let word = &before[start..];
        if word.is_empty() {
            return Ok((pos, Vec::new()));
        }
        let candidates = self
            .names
            .borrow()
            .iter()
            .filter(|name| name.starts_with(word))
            .cloned()
            .map(pair)
            .collect();
        Ok((start, candidates))
    }
}

fn pair(text: String) -> Pair {
    Pair {
        display: text.clone(),
        replacement: text,
    }
}

/// Remembers the last byte written through it.
struct Tracking {
    inner: Box<dyn Write>,
    last: Rc<Cell<Option<u8>>>,
}

impl Write for Tracking {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        if written > 0 {
            self.last.set(Some(buf[written - 1]));
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::SharedBuffer;

    struct Harness {
        repl: Repl,
        program: SharedBuffer,
        out: SharedBuffer,
        err: SharedBuffer,
    }

    fn harness() -> Harness {
        let (program, out, err) = (
            SharedBuffer::new(),
            SharedBuffer::new(),
            SharedBuffer::new(),
        );
        let repl = Repl::new(
            Box::new(program.clone()),
            Box::new(out.clone()),
            Box::new(err.clone()),
            false,
        );
        Harness {
            repl,
            program,
            out,
            err,
        }
    }

    #[test]
    fn definitions_report_their_types_and_values_print() {
        let mut h = harness();
        h.repl.handle(r"double = \x. (mul x 2)");
        h.repl.handle("(double 21)");
        assert_eq!(h.out.take(), "double : Number -> Number\n=> 42 : Number\n");
        assert_eq!(h.err.take(), "");
    }

    #[test]
    fn unit_results_are_not_echoed_and_output_ends_its_line() {
        let mut h = harness();
        h.repl.handle(r#"(print "hi")"#);
        assert_eq!(h.program.take(), "hi");
        assert_eq!(h.out.take(), "\n");
        h.repl.handle(r#"(println "hi")"#);
        assert_eq!(h.out.take(), "");
    }

    #[test]
    fn errors_go_to_the_error_stream() {
        let mut h = harness();
        h.repl.handle("(add 1 \"x\")");
        let err = h.err.take();
        assert!(err.contains("mismatched types"), "{err}");
        assert!(err.contains("<repl>"), "{err}");
        assert_eq!(h.out.take(), "");
    }

    #[test]
    fn type_env_reset_and_docs_commands() {
        let mut h = harness();
        h.repl.handle(":type map");
        assert_eq!(h.out.take(), "map : (a -> b) -> List<a> -> List<b>\n");
        h.repl.handle(":env");
        assert_eq!(h.out.take(), "no definitions yet\n");
        h.repl.handle("x = 1");
        h.repl.handle(":e");
        assert_eq!(h.out.take(), "x : Number\nx : Number\n");
        h.repl.handle(":reset");
        assert_eq!(
            h.out.take(),
            "environment cleared
"
        );
        h.repl.handle("x");
        assert!(h.err.take().contains("cannot find `x`"));
        h.repl.handle(":docs head");
        assert!(h.out.take().starts_with("head : List<a> -> a\n"));
        h.repl.handle(":docs nope");
        assert_eq!(h.err.take(), "no builtin named `nope`\n");
    }

    #[test]
    fn load_command() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("lib.lbd");
        std::fs::write(&file, "triple = \\x. (mul x 3)\n(print \"loading\")").unwrap();
        let mut h = harness();
        h.repl.handle(&format!(":load {}", file.display()));
        assert_eq!(h.program.take(), "loading");
        assert!(h.out.take().ends_with("(1 definition)\n"));
        h.repl.handle("(triple 2)");
        assert!(h.out.take().contains("=> 6 : Number"));
    }

    #[test]
    fn quit_and_unknown_commands() {
        let mut h = harness();
        assert_eq!(h.repl.handle(":quit"), Control::Quit);
        assert_eq!(h.repl.handle(":frobnicate"), Control::Continue);
        assert!(h.err.take().contains("unknown command `:frobnicate`"));
        h.repl.handle(":help");
        assert!(h.out.take().contains(":type <expr>"));
        h.repl.handle(":type");
        assert_eq!(
            h.err.take(),
            "usage: :type <expr>
"
        );
    }

    #[test]
    fn commands_accept_unique_prefixes() {
        let mut h = harness();
        h.repl.handle(":t (add 1)");
        h.repl.handle(":ty True");
        assert_eq!(
            h.out.take(),
            "(add 1) : Number -> Number
True : Bool
"
        );
        assert_eq!(h.repl.handle(":q"), Control::Quit);
    }

    #[test]
    fn info_shows_types_and_origins() {
        let mut h = harness();
        h.repl.handle(r"square = \x. (mul x x)");
        h.out.take();
        h.repl.handle(":info square");
        assert_eq!(
            h.out.take(),
            "square : Number -> Number
    defined at <repl>:1:1
"
        );
        h.repl.handle(":i map");
        assert!(h.out.take().starts_with(
            "map : (a -> b) -> List<a> -> List<b>
"
        ));
        h.repl
            .handle("type Shape =\n    | Circle Number\n    | Pair (Number -> Number) List<Shape>");
        h.out.take();
        h.repl.handle(":info Shape");
        assert_eq!(
            h.out.take(),
            "type Shape = Circle Number | Pair (Number -> Number) List<Shape>
    defined at <repl>:1:6
"
        );
        h.repl.handle(":info Circle");
        assert_eq!(
            h.out.take(),
            "Circle : Number -> Shape
    a constructor of `Shape`
    defined at <repl>:2:7
"
        );
        h.repl.handle(":info Option");
        assert!(h.err.take().contains("`Option` is not defined"));
        h.repl.handle(":info True");
        assert_eq!(h.out.take(), "True : Bool\n    a constructor of `Bool`\n");
        h.repl.handle(":info nothing");
        assert_eq!(
            h.err.take(),
            "`nothing` is not defined
"
        );
    }

    #[test]
    fn reload_picks_up_changes() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("lib.lbd");
        let mut h = harness();
        h.repl.handle(":reload");
        assert_eq!(
            h.err.take(),
            "nothing to reload; use :load <file> first
"
        );
        std::fs::write(&file, "answer = 1").unwrap();
        h.repl.handle(&format!(":l {}", file.display()));
        std::fs::write(&file, "answer = 42").unwrap();
        h.repl.handle(":r");
        h.out.take();
        h.repl.handle("answer");
        assert_eq!(
            h.out.take(),
            "=> 42 : Number
"
        );
    }

    #[test]
    fn completeness_detection() {
        assert!(is_complete("(add 1 2)"));
        assert!(!is_complete("(add 1"));
        assert!(!is_complete("[1,"));
        assert!(!is_complete("f = \\x."));
        assert!(!is_complete("f ="));
        assert!(!is_complete("f : a ->"));
        assert!(!is_complete("x = let y = 1 in"));
        assert!(!is_complete("x = let y <-"));
        assert!(!is_complete("\"unterminated"));
        assert!(!is_complete("type Shape ="));
        assert!(!is_complete("type Shape = Circle Number |"));
        assert!(!is_complete("f = \\s. match s"));
        assert!(!is_complete("f = \\s. match s | Circle r ->"));
        assert!(is_complete("f = \\s. match s | Circle r -> r"));
        assert!(is_complete(":type (add"));
        assert!(is_complete("x = 1 -- comment ("));
        assert!(
            is_complete("(add 1))"),
            "too many closers is an error for the parser to report"
        );
    }

    #[test]
    fn line_mode_joins_multi_line_input() {
        let mut h = harness();
        let input = "f = \\x.\n  (add x\n     1)\n(f 1)\n:quit\n(f 2)\n";
        run_lines(&mut h.repl, input.as_bytes()).unwrap();
        assert_eq!(h.out.take(), "f : Number -> Number\n=> 2 : Number\n");
    }

    #[test]
    fn completion_names_include_definitions() {
        let mut h = harness();
        h.repl.handle("myValue = 1");
        let names = h.repl.completion_names();
        assert!(names.contains(&"myValue".to_owned()));
        assert!(names.contains(&"foldr".to_owned()));
    }
}
