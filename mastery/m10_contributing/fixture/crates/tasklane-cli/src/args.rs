//! Command-line parsing. Hand written: the command line is small, and the project
//! keeps its dependency list short.

use bitflags::bitflags;

bitflags! {
    /// The parts of the output that `tasklane run` prints.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Show: u8 {
        /// The workspace log.
        const LOG = 0b001;
        /// The list of files with their sizes.
        const FILES = 0b010;
        /// The one-line summary.
        const SUMMARY = 0b100;
    }
}

/// What the user asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `tasklane list`
    List,
    /// `tasklane plan TASK`
    Plan(String),
    /// `tasklane run TASK`
    Run(String),
    /// `tasklane check`
    Check,
    /// `tasklane steps`
    Steps,
    /// `tasklane --help`
    Help,
    /// `tasklane --version`
    Version,
}

/// A parsed command line.
///
/// A new option adds a field. The struct is `#[non_exhaustive]` so that this breaks
/// nobody: code outside this crate gets a `Cli` from [`parse`] and cannot build one
/// with a struct literal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Cli {
    /// The command.
    pub command: Command,
    /// The path given with `--file`, or `Lanefile`.
    pub file: String,
    /// The `--set NAME=VALUE` pairs, in order.
    pub sets: Vec<(String, String)>,
    /// `--keep-going`
    pub keep_going: bool,
    /// What `run` prints.
    pub show: Show,
}

/// The text printed by `--help`.
pub const HELP: &str = "\
Usage: tasklane [OPTIONS] COMMAND

Commands:
  list         show the tasks of the Lanefile
  plan TASK    show what `run TASK` would run, in order
  run TASK     run a task and everything it depends on
  check        parse the Lanefile and plan every task
  steps        show the available steps

Options:
  -f, --file PATH       the Lanefile to read (default: Lanefile)
      --set NAME=VALUE  set or override a variable; may be repeated
  -k, --keep-going      after a failure, still run tasks that do not depend on it
      --show PARTS      what `run` prints: log, files, summary, separated by
                        commas (default: log,summary)
  -q, --quiet           the same as --show summary
  -h, --help            print this text
  -V, --version         print the version
";

/// Parses the arguments after the program name.
///
/// The error is a message for the user, without the `error:` prefix.
pub fn parse(args: &[String]) -> Result<Cli, String> {
    let mut cli = Cli {
        command: Command::Help,
        file: "Lanefile".to_string(),
        sets: Vec::new(),
        keep_going: false,
        show: Show::LOG | Show::SUMMARY,
    };
    let mut words: Vec<&str> = Vec::new();
    let mut iter = args.iter().map(String::as_str);
    while let Some(arg) = iter.next() {
        match arg {
            "-h" | "--help" => {
                cli.command = Command::Help;
                return Ok(cli);
            }
            "-V" | "--version" => {
                cli.command = Command::Version;
                return Ok(cli);
            }
            "-k" | "--keep-going" => cli.keep_going = true,
            "-q" | "--quiet" => cli.show = Show::SUMMARY,
            "-f" | "--file" => cli.file = value(arg, iter.next())?.to_string(),
            "--set" => cli.sets.push(parse_set(value(arg, iter.next())?)?),
            "--show" => cli.show = parse_show(value(arg, iter.next())?)?,
            option if option.starts_with('-') && option.len() > 1 => {
                return Err(format!("unknown option `{option}`"));
            }
            word => words.push(word),
        }
    }
    cli.command = match words.as_slice() {
        [] => return Err("missing command; try --help".to_string()),
        ["list"] => Command::List,
        ["check"] => Command::Check,
        ["steps"] => Command::Steps,
        ["plan", task] => Command::Plan(task.to_string()),
        ["run", task] => Command::Run(task.to_string()),
        ["plan" | "run"] => return Err(format!("`{}` needs a task name", words[0])),
        ["list" | "check" | "steps" | "plan" | "run", ..] => {
            return Err(format!("too many arguments for `{}`", words[0]));
        }
        [other, ..] => return Err(format!("unknown command `{other}`")),
    };
    Ok(cli)
}

fn value<'a>(option: &str, next: Option<&'a str>) -> Result<&'a str, String> {
    next.ok_or_else(|| format!("`{option}` needs a value"))
}

fn parse_set(text: &str) -> Result<(String, String), String> {
    match text.split_once('=') {
        Some((name, value)) if tasklane_core::text::is_name(name) => {
            Ok((name.to_string(), value.to_string()))
        }
        _ => Err(format!("expected NAME=VALUE after `--set`, found `{text}`")),
    }
}

fn parse_show(text: &str) -> Result<Show, String> {
    let mut show = Show::empty();
    for part in text.split(',') {
        show |= match part.trim() {
            "log" => Show::LOG,
            "files" => Show::FILES,
            "summary" => Show::SUMMARY,
            other => return Err(format!("unknown output part `{other}`")),
        };
    }
    Ok(show)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli(args: &[&str]) -> Result<Cli, String> {
        let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        parse(&args)
    }

    #[test]
    fn defaults() {
        let cli = cli(&["list"]).unwrap();
        assert_eq!(cli.command, Command::List);
        assert_eq!(cli.file, "Lanefile");
        assert!(!cli.keep_going);
        assert_eq!(cli.show, Show::LOG | Show::SUMMARY);
    }

    #[test]
    fn options_may_come_before_or_after_the_command() {
        let before = cli(&["-f", "a.lane", "-k", "run", "site"]).unwrap();
        let after = cli(&["run", "site", "--keep-going", "--file", "a.lane"]).unwrap();
        assert_eq!(before, after);
        assert_eq!(before.command, Command::Run("site".to_string()));
        assert_eq!(before.file, "a.lane");
    }

    #[test]
    fn set_may_repeat_and_values_may_contain_equals() {
        let cli = cli(&["--set", "A=1", "--set", "B=x=y", "check"]).unwrap();
        assert_eq!(
            cli.sets,
            [
                ("A".to_string(), "1".to_string()),
                ("B".to_string(), "x=y".to_string())
            ]
        );
    }

    #[test]
    fn show_and_quiet() {
        assert_eq!(
            cli(&["--show", "files", "run", "t"]).unwrap().show,
            Show::FILES
        );
        assert_eq!(
            cli(&["--show", "files, log", "run", "t"]).unwrap().show,
            Show::FILES | Show::LOG
        );
        assert_eq!(cli(&["-q", "run", "t"]).unwrap().show, Show::SUMMARY);
        assert_eq!(
            cli(&["--show", "all", "run", "t"]).unwrap_err(),
            "unknown output part `all`"
        );
    }

    #[test]
    fn usage_errors() {
        assert_eq!(cli(&[]).unwrap_err(), "missing command; try --help");
        assert_eq!(cli(&["run"]).unwrap_err(), "`run` needs a task name");
        assert_eq!(
            cli(&["list", "x"]).unwrap_err(),
            "too many arguments for `list`"
        );
        assert_eq!(cli(&["build"]).unwrap_err(), "unknown command `build`");
        assert_eq!(
            cli(&["--fast", "list"]).unwrap_err(),
            "unknown option `--fast`"
        );
        assert_eq!(cli(&["list", "-f"]).unwrap_err(), "`-f` needs a value");
        assert_eq!(
            cli(&["--set", "novalue", "list"]).unwrap_err(),
            "expected NAME=VALUE after `--set`, found `novalue`"
        );
    }

    #[test]
    fn help_and_version_win() {
        assert_eq!(cli(&["run", "--help"]).unwrap().command, Command::Help);
        assert_eq!(cli(&["-V"]).unwrap().command, Command::Version);
    }
}
