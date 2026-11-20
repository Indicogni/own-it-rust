//! The command line (SPEC 7.1).

use std::ffi::OsString;
use std::fmt;
use std::num::NonZeroUsize;

/// The text that `--help` prints.
pub const HELP: &str = "\
Usage: rgx [OPTIONS] PATTERN [PATH...]

Prints the lines that contain a match of PATTERN. A PATH is a file, or a
directory that is searched with everything below it. Without a PATH, rgx
reads standard input.

Options:
  -i, --ignore-case          ignore the case of the letters A to Z
  -v, --invert-match         select the lines that do not match
  -n, --line-number          put the line number before each line
  -c, --count                print only the number of selected lines per file
  -l, --files-with-matches   print only the names of files with selected lines
  -o, --only-matching        print each match on a line of its own
  -j, --threads N            search with N threads (1 to 256)
  -h, --help                 print this text
  --                         everything after this is PATTERN or PATH

Exit status: 0 if a line was selected, 1 if none was, 2 after an error.
";

/// The largest value of `--threads`.
const MAX_THREADS: usize = 256;

/// What to print for the selected lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The lines themselves.
    Lines,
    /// `-c`: how many there are in each file.
    Count,
    /// `-l`: the names of the files that have any.
    FilesWithMatches,
    /// `-o`: the matches in them.
    OnlyMatching,
}

/// A complete, checked request to search.
#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    pub pattern: String,
    /// Empty means standard input.
    pub paths: Vec<OsString>,
    pub ignore_case: bool,
    pub invert: bool,
    pub line_numbers: bool,
    pub mode: Mode,
    /// `None`: as many threads as the machine offers.
    pub threads: Option<NonZeroUsize>,
}

/// What the arguments ask for.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Search(Config),
}

/// A command line that cannot be carried out. The text completes the
/// sentence "rgx: usage error: ...".
#[derive(Debug, PartialEq, Eq)]
pub struct UsageError(String);

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for UsageError {}

fn usage<T>(message: impl Into<String>) -> Result<T, UsageError> {
    Err(UsageError(message.into()))
}

/// Parses the arguments after the program name.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, UsageError> {
    let args: Vec<OsString> = args.into_iter().collect();
    // SPEC 7.1: a request for help anywhere among the options wins, and
    // nothing else is checked.
    let options = args.iter().take_while(|arg| *arg != "--");
    if options
        .into_iter()
        .any(|arg| arg == "-h" || arg == "--help")
    {
        return Ok(Command::Help);
    }
    let mut args = args.into_iter();
    let mut operands: Vec<OsString> = Vec::new();
    let mut ignore_case = false;
    let mut invert = false;
    let mut line_numbers = false;
    let mut mode = None;
    let mut threads = None;

    while let Some(arg) = args.next() {
        // An option is an argument of at least two characters that starts
        // with `-`. A lone `-` and anything that is not UTF-8 are operands
        // (no option name is anything but ASCII).
        let option = match arg.to_str() {
            Some(text) if text.len() > 1 && text.starts_with('-') => text,
            _ => {
                operands.push(arg);
                continue;
            }
        };
        match option {
            "--" => {
                operands.extend(args);
                break;
            }
            "-i" | "--ignore-case" => ignore_case = true,
            "-v" | "--invert-match" => invert = true,
            "-n" | "--line-number" => line_numbers = true,
            "-c" | "--count" => set_mode(&mut mode, Mode::Count, "--count")?,
            "-l" | "--files-with-matches" => {
                set_mode(&mut mode, Mode::FilesWithMatches, "--files-with-matches")?
            }
            "-o" | "--only-matching" => set_mode(&mut mode, Mode::OnlyMatching, "--only-matching")?,
            "-j" | "--threads" => {
                let Some(value) = args.next() else {
                    return usage(format!("{option} needs a number"));
                };
                threads = Some(parse_threads(&value)?);
            }
            unknown => return usage(format!("unknown option {unknown} (try --help)")),
        }
    }

    let mode = mode.map_or(Mode::Lines, |(mode, _)| mode);
    if mode == Mode::OnlyMatching && invert {
        return usage("--only-matching and --invert-match cannot be used together");
    }

    let mut operands = operands.into_iter();
    let Some(pattern) = operands.next() else {
        return usage("no pattern given (try --help)");
    };
    let Ok(pattern) = pattern.into_string() else {
        return usage("the pattern is not valid UTF-8");
    };
    Ok(Command::Search(Config {
        pattern,
        paths: operands.collect(),
        ignore_case,
        invert,
        line_numbers,
        mode,
        threads,
    }))
}

/// Records the output mode. Giving the same one twice is harmless; two
/// different ones contradict each other.
fn set_mode(
    slot: &mut Option<(Mode, &'static str)>,
    mode: Mode,
    name: &'static str,
) -> Result<(), UsageError> {
    match *slot {
        Some((earlier, earlier_name)) if earlier != mode => {
            usage(format!("{earlier_name} and {name} cannot be used together"))
        }
        _ => {
            *slot = Some((mode, name));
            Ok(())
        }
    }
}

fn parse_threads(value: &OsString) -> Result<NonZeroUsize, UsageError> {
    value
        .to_str()
        // `str::parse` would also take a sign: `+4` is not a number here.
        .filter(|text| text.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|text| text.parse::<NonZeroUsize>().ok())
        .filter(|n| n.get() <= MAX_THREADS)
        .ok_or_else(|| {
            UsageError(format!(
                "the number of threads must be from 1 to {MAX_THREADS}, not {}",
                value.to_string_lossy()
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(args: &[&str]) -> Result<Command, UsageError> {
        parse(args.iter().map(OsString::from))
    }

    fn config(args: &[&str]) -> Config {
        match parse_strs(args) {
            Ok(Command::Search(config)) => config,
            other => panic!("{args:?} parsed as {other:?}"),
        }
    }

    #[test]
    fn options_may_stand_anywhere_before_the_double_dash() {
        let config = config(&["-n", "needle", "a.txt", "--ignore-case", "b.txt", "-j", "3"]);
        assert_eq!(config.pattern, "needle");
        assert_eq!(config.paths, ["a.txt", "b.txt"]);
        assert!(config.ignore_case && config.line_numbers && !config.invert);
        assert_eq!(config.threads.map(NonZeroUsize::get), Some(3));
        assert_eq!(config.mode, Mode::Lines);
    }

    #[test]
    fn after_the_double_dash_nothing_is_an_option() {
        let config = config(&["-c", "--", "-v", "-n"]);
        assert_eq!(
            (config.pattern.as_str(), config.mode, config.invert),
            ("-v", Mode::Count, false)
        );
        assert_eq!(config.paths, ["-n"]);
    }

    #[test]
    fn a_lone_dash_is_an_operand() {
        assert_eq!(config(&["-", "x"]).pattern, "-");
    }

    #[test]
    fn help_wins() {
        assert_eq!(parse_strs(&["x", "--help"]), Ok(Command::Help));
        assert_eq!(parse_strs(&["-h"]), Ok(Command::Help));
        assert_eq!(parse_strs(&["--bogus", "-j", "--help"]), Ok(Command::Help));
        assert_eq!(config(&["--", "--help"]).pattern, "--help");
    }

    #[test]
    fn usage_errors() {
        for args in [
            &[][..],
            &["-n"],
            &["--frobnicate", "x"],
            &["-in", "x"],
            &["-j"],
            &["x", "-j", "0"],
            &["x", "-j", "257"],
            &["x", "-j", "two"],
            &["x", "-j", "+2"],
            &["x", "-j", ""],
            &["x", "-j4"],
            &["x", "--threads=4"],
            &["-c", "-l", "x"],
            &["-o", "-c", "x"],
            &["-l", "-o", "x"],
            &["-o", "-v", "x"],
        ] {
            assert!(parse_strs(args).is_err(), "{args:?} must be a usage error");
        }
    }

    #[test]
    fn a_repeated_mode_is_not_a_conflict() {
        assert_eq!(config(&["-c", "--count", "x"]).mode, Mode::Count);
        assert_eq!(
            config(&["x", "-j", "2", "-j", "5"])
                .threads
                .map(NonZeroUsize::get),
            Some(5)
        );
    }
}
