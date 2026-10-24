//! REPL meta-commands.
//!
//! Commands start with `:`, the convention of `GHCi`, the OCaml and Scala
//! REPLs, and other functional-language REPLs. As in `GHCi`, any prefix of a
//! command's name works, and when a prefix fits several commands the one
//! listed first wins, so the common ones get the shortest spellings
//! (`:t`, `:l`, `:r`).

/// A REPL command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Help,
    Quit,
    Type,
    Info,
    Load,
    Reload,
    Env,
    Docs,
    Reset,
}

pub struct Spec {
    pub command: Command,
    pub name: &'static str,
    /// Extra spellings that are not prefixes of `name`.
    pub aliases: &'static [&'static str],
    pub argument: &'static str,
    pub summary: &'static str,
}

/// Every command, in prefix-priority order.
pub const COMMANDS: &[Spec] = &[
    Spec {
        command: Command::Help,
        name: "help",
        aliases: &["?"],
        argument: "",
        summary: "show this help",
    },
    Spec {
        command: Command::Quit,
        name: "quit",
        aliases: &["exit"],
        argument: "",
        summary: "leave the REPL (or press Ctrl-D)",
    },
    Spec {
        command: Command::Type,
        name: "type",
        aliases: &[],
        argument: "<expr>",
        summary: "show the type of an expression without evaluating it",
    },
    Spec {
        command: Command::Info,
        name: "info",
        aliases: &[],
        argument: "<name>",
        summary: "show the type and origin of a definition or builtin",
    },
    Spec {
        command: Command::Load,
        name: "load",
        aliases: &[],
        argument: "<file>",
        summary: "load and run a file",
    },
    Spec {
        command: Command::Reload,
        name: "reload",
        aliases: &[],
        argument: "",
        summary: "load the last loaded file again",
    },
    Spec {
        command: Command::Env,
        name: "env",
        aliases: &["browse"],
        argument: "",
        summary: "list your definitions and their types",
    },
    Spec {
        command: Command::Docs,
        name: "docs",
        aliases: &[],
        argument: "[name]",
        summary: "show builtin documentation",
    },
    Spec {
        command: Command::Reset,
        name: "reset",
        aliases: &[],
        argument: "",
        summary: "forget every definition",
    },
];

/// Resolves what the user typed after `:`.
pub fn lookup(typed: &str) -> Option<Command> {
    if typed.is_empty() {
        return None;
    }
    COMMANDS
        .iter()
        .find(|spec| spec.name == typed || spec.aliases.contains(&typed))
        .or_else(|| COMMANDS.iter().find(|spec| spec.name.starts_with(typed)))
        .map(|spec| spec.command)
}

/// `:name` completions for a partially typed command (without the `:`).
pub fn completions(typed: &str) -> Vec<String> {
    COMMANDS
        .iter()
        .filter(|spec| spec.name.starts_with(typed))
        .map(|spec| format!(":{}", spec.name))
        .collect()
}

/// The `:help` text.
pub fn help() -> String {
    let usage: Vec<String> = COMMANDS
        .iter()
        .map(|spec| {
            let mut usage = format!(":{}", spec.name);
            if !spec.argument.is_empty() {
                usage.push(' ');
                usage.push_str(spec.argument);
            }
            usage
        })
        .collect();
    let width = usage.iter().map(String::len).max().unwrap_or(0);
    let mut out = String::from("Commands (any unique prefix works, e.g. :t for :type):\n");
    for (spec, usage) in COMMANDS.iter().zip(&usage) {
        out.push_str(&format!("  {usage:width$}  {}\n", spec.summary));
    }
    out.push_str(
        "\nEnter definitions (`name = expr`) or expressions. Input continues on the\n\
         next line while brackets are open or a line ends with `\\`, `.`, `=` or `->`.\n\
         Tab completes names, commands and, after :load, file paths.",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_names_and_aliases() {
        for spec in COMMANDS {
            assert_eq!(lookup(spec.name), Some(spec.command));
            for alias in spec.aliases {
                assert_eq!(lookup(alias), Some(spec.command));
            }
        }
    }

    #[test]
    fn prefixes_follow_priority_order() {
        assert_eq!(lookup("t"), Some(Command::Type));
        assert_eq!(lookup("l"), Some(Command::Load));
        assert_eq!(lookup("r"), Some(Command::Reload));
        assert_eq!(lookup("res"), Some(Command::Reset));
        assert_eq!(lookup("q"), Some(Command::Quit));
        assert_eq!(lookup("i"), Some(Command::Info));
        assert_eq!(lookup("e"), Some(Command::Env));
        assert_eq!(lookup("d"), Some(Command::Docs));
    }

    #[test]
    fn unknown_commands() {
        assert_eq!(lookup(""), None);
        assert_eq!(lookup("frobnicate"), None);
        assert_eq!(lookup("types"), None);
    }

    #[test]
    fn completion_lists_matching_commands() {
        assert_eq!(completions("re"), [":reload", ":reset"]);
        assert_eq!(completions("ty"), [":type"]);
        assert_eq!(completions("").len(), COMMANDS.len());
    }

    #[test]
    fn help_mentions_every_command() {
        let text = help();
        for spec in COMMANDS {
            assert!(text.contains(&format!(":{}", spec.name)), "{}", spec.name);
            assert!(text.contains(spec.summary));
        }
    }
}
