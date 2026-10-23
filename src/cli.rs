//! Command-line interface.

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::builtins::{Builtin, reference, reference_json};
use crate::diagnostics::stderr_supports_color;
use crate::session::{Session, Stage};

#[derive(Debug, Parser)]
#[command(
    name = "lbd",
    version,
    about = "Lambda Discipline: a lazy, statically typed functional language"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Start the REPL (the same as `lbd repl`; kept for editor integrations).
    #[arg(long, hide = true)]
    pub repl: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run a program
    Run {
        /// The program to run
        file: PathBuf,
        /// Treat warnings as errors, and do not run a program with any
        #[arg(long)]
        deny_warnings: bool,
    },
    /// Type-check a program without running it
    Check {
        /// The program to check
        file: PathBuf,
        /// Print an intermediate representation instead
        #[arg(long, value_enum)]
        emit: Option<Stage>,
        /// Treat warnings as errors
        #[arg(long)]
        deny_warnings: bool,
    },
    /// Start an interactive session (the default when no command is given)
    Repl,
    /// Show the reference for builtin functions
    Docs {
        /// Only show this builtin
        name: Option<String>,
        /// Print JSON for tools instead of text
        #[arg(long)]
        json: bool,
    },
}

/// The compiler front end recurses over expression nesting, so it runs on a
/// thread with a generous stack (reserved address space, not committed memory).
/// The parser bounds that nesting (`syntax::parser::MAX_NESTING`) well within
/// this stack. The interpreter itself does not use the native stack for recursion.
const STACK_SIZE: usize = 256 * 1024 * 1024;

pub fn main() -> ExitCode {
    let cli = Cli::parse();
    std::thread::Builder::new()
        .name("lbd".into())
        .stack_size(STACK_SIZE)
        .spawn(move || run(cli))
        .expect("cannot start the main thread")
        .join()
        .unwrap_or(ExitCode::FAILURE)
}

pub fn run(cli: Cli) -> ExitCode {
    let color = stderr_supports_color();
    let command = match (cli.command, cli.repl) {
        (Some(command), _) => command,
        (None, _) => Command::Repl,
    };
    match command {
        Command::Run {
            file,
            deny_warnings,
        } => {
            let mut session = Session::new(Box::new(io::stdout()));
            session.set_deny_warnings(deny_warnings);
            // Warnings come before anything the program prints.
            let result = session.check_file(&file).and_then(|compiled| {
                warn(&mut session, color);
                session.run(&compiled)
            });
            session.flush();
            report(&session, result.map(drop), color)
        }
        Command::Check {
            file,
            emit: None,
            deny_warnings,
        } => {
            let mut session = Session::new(Box::new(io::sink()));
            session.set_deny_warnings(deny_warnings);
            let result = session.check_file(&file);
            warn(&mut session, color);
            report(&session, result.map(drop), color)
        }
        Command::Check {
            file,
            emit: Some(stage),
            deny_warnings,
        } => {
            let mut session = Session::new(Box::new(io::sink()));
            session.set_deny_warnings(deny_warnings);
            let result = session.emit(&file, stage);
            if let Ok(text) = &result {
                print!("{text}");
            }
            report(&session, result.map(drop), color)
        }
        Command::Repl => match crate::repl::run(color) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        Command::Docs { name: None, json } => {
            print!("{}", if json { reference_json() } else { reference() });
            ExitCode::SUCCESS
        }
        Command::Docs {
            name: Some(name),
            json,
        } => match Builtin::lookup(&name) {
            Some(builtin) if json => {
                println!("{}", builtin.to_json());
                ExitCode::SUCCESS
            }
            Some(builtin) => {
                println!(
                    "{} : {}\n    {}",
                    builtin.name(),
                    builtin.signature(),
                    builtin.doc()
                );
                ExitCode::SUCCESS
            }
            None => {
                eprintln!("error: no builtin named `{name}`");
                ExitCode::FAILURE
            }
        },
    }
}

/// Prints the warnings compiling has produced so far.
fn warn(session: &mut Session, color: bool) {
    let warnings = session.take_warnings();
    session.render(&warnings, &mut io::stderr(), color);
}

fn report(
    session: &Session,
    result: Result<(), Vec<crate::diagnostics::Diagnostic>>,
    color: bool,
) -> ExitCode {
    let _ = io::stdout().flush();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(diagnostics) => {
            session.render(&diagnostics, &mut io::stderr(), color);
            ExitCode::FAILURE
        }
    }
}
