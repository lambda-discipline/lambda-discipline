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
    },
    /// Type-check a program without running it
    Check {
        /// The program to check
        file: PathBuf,
        /// Print an intermediate representation instead
        #[arg(long, value_enum)]
        emit: Option<Stage>,
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
/// The interpreter itself does not use the native stack for recursion.
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
        Command::Run { file } => {
            let mut session = Session::new(Box::new(io::stdout()));
            let result = session.run_file(&file);
            session.flush();
            report(&session, result.map(drop), color)
        }
        Command::Check { file, emit: None } => {
            let mut session = Session::new(Box::new(io::sink()));
            let result = session.check_file(&file);
            report(&session, result.map(drop), color)
        }
        Command::Check {
            file,
            emit: Some(stage),
        } => {
            let mut session = Session::new(Box::new(io::sink()));
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
                    builtin.display_signature(),
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
