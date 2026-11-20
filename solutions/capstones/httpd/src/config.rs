//! The command line (SPEC section 3).
//!
//! Parsing is a pure function from arguments to a [`Config`], so every rule
//! can be unit tested without starting a process.

use std::ffi::OsString;
use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

/// The text `--help` prints.
pub const USAGE: &str = "\
Usage: httpd --root DIR [OPTIONS]

Serves the files below DIR over HTTP/1.1 and echoes JSON posted to /echo.
Write the line `shutdown` to standard input, or close it, to stop.

Options:
  --root DIR                 directory to serve (required)
  --addr IP:PORT             address to listen on [127.0.0.1:8080]
  --workers N                worker threads, 1..=256 [4]
  --queue N                  connections that may wait for a worker, 0..=4096 [16]
  --max-body BYTES           largest request body, 0..=16777216 [1048576]
  --request-timeout-ms MS    time allowed for one request, 1..=600000 [5000]
  --idle-timeout-ms MS       time a connection may wait for a request, 1..=600000 [5000]
  --help                     print this text";

/// Everything the server needs to know before it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub root: PathBuf,
    pub addr: SocketAddr,
    pub workers: usize,
    pub queue: usize,
    pub max_body: u64,
    pub request_timeout: Duration,
    pub idle_timeout: Duration,
}

/// What the command line asked for.
#[derive(Debug, PartialEq, Eq)]
pub enum Cli {
    Run(Config),
    Help,
}

/// A command line the server refuses, with the reason.
#[derive(Debug, PartialEq, Eq)]
pub enum UsageError {
    MissingRoot,
    UnknownArgument(String),
    MissingValue(&'static str),
    Repeated(&'static str),
    BadValue {
        option: &'static str,
        value: String,
        expected: String,
    },
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRoot => write!(f, "the option --root DIR is required"),
            Self::UnknownArgument(arg) => write!(f, "unknown argument `{arg}`"),
            Self::MissingValue(option) => write!(f, "{option} needs a value"),
            Self::Repeated(option) => write!(f, "{option} was given more than once"),
            Self::BadValue {
                option,
                value,
                expected,
            } => write!(f, "{option}: `{value}` is not {expected}"),
        }
    }
}

impl std::error::Error for UsageError {}

/// The options that take a value, as written on the command line.
const OPTIONS: [&str; 7] = [
    "--root",
    "--addr",
    "--workers",
    "--queue",
    "--max-body",
    "--request-timeout-ms",
    "--idle-timeout-ms",
];

/// Parses the arguments after the program name.
///
/// `--help` wins over everything else, even over errors elsewhere on the
/// line: a user who asks for help should get it.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Cli, UsageError> {
    let args: Vec<OsString> = args.into_iter().collect();
    if args.iter().any(|arg| arg == "--help") {
        return Ok(Cli::Help);
    }

    // One slot per entry of OPTIONS. Values stay `OsString` so that a root
    // directory whose name is not UTF-8 still works.
    let mut values: [Option<OsString>; OPTIONS.len()] = Default::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let Some(index) = OPTIONS.iter().position(|option| arg == *option) else {
            return Err(UsageError::UnknownArgument(
                arg.to_string_lossy().into_owned(),
            ));
        };
        let option = OPTIONS[index];
        let value = args.next().ok_or(UsageError::MissingValue(option))?;
        if values[index].replace(value).is_some() {
            return Err(UsageError::Repeated(option));
        }
    }
    let [
        root,
        addr,
        workers,
        queue,
        max_body,
        request_timeout,
        idle_timeout,
    ] = values;

    let root = PathBuf::from(root.ok_or(UsageError::MissingRoot)?);
    let addr = match addr {
        None => SocketAddr::from(([127, 0, 0, 1], 8080)),
        Some(value) => {
            let text = value.to_string_lossy();
            text.parse().map_err(|_| UsageError::BadValue {
                option: "--addr",
                value: text.to_string(),
                expected: "an address such as 127.0.0.1:8080".into(),
            })?
        }
    };
    let workers = number("--workers", workers, 4, 1, 256)?;
    let queue = number("--queue", queue, 16, 0, 4096)?;
    let max_body = number("--max-body", max_body, 1 << 20, 0, 1 << 24)?;
    let request_timeout = number("--request-timeout-ms", request_timeout, 5000, 1, 600_000)?;
    let idle_timeout = number("--idle-timeout-ms", idle_timeout, 5000, 1, 600_000)?;

    Ok(Cli::Run(Config {
        root,
        addr,
        // Both fit: the ranges above are far below `usize::MAX` on every
        // platform Rust's standard library supports.
        workers: workers as usize,
        queue: queue as usize,
        max_body,
        request_timeout: Duration::from_millis(request_timeout),
        idle_timeout: Duration::from_millis(idle_timeout),
    }))
}

/// A decimal number in `min..=max`, or `default` when the option is absent.
fn number(
    option: &'static str,
    value: Option<OsString>,
    default: u64,
    min: u64,
    max: u64,
) -> Result<u64, UsageError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let text = value.to_string_lossy();
    // `u64::from_str` accepts a leading `+`; the SPEC allows digits only.
    let parsed = if text.bytes().all(|b| b.is_ascii_digit()) {
        text.parse::<u64>().ok()
    } else {
        None
    };
    match parsed {
        Some(n) if (min..=max).contains(&n) => Ok(n),
        _ => Err(UsageError::BadValue {
            option,
            value: text.to_string(),
            expected: format!("a whole number from {min} to {max}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli(args: &[&str]) -> Result<Cli, UsageError> {
        parse(args.iter().map(OsString::from))
    }

    fn config(args: &[&str]) -> Config {
        match cli(args) {
            Ok(Cli::Run(config)) => config,
            other => panic!("{args:?} should be accepted, got {other:?}"),
        }
    }

    #[test]
    fn defaults_match_the_spec() {
        let config = config(&["--root", "site"]);
        assert_eq!(config.root, PathBuf::from("site"));
        assert_eq!(config.addr, "127.0.0.1:8080".parse().unwrap());
        assert_eq!((config.workers, config.queue), (4, 16));
        assert_eq!(config.max_body, 1_048_576);
        assert_eq!(config.request_timeout, Duration::from_secs(5));
        assert_eq!(config.idle_timeout, Duration::from_secs(5));
    }

    #[test]
    fn every_option_is_read() {
        let config = config(&[
            "--idle-timeout-ms",
            "7",
            "--request-timeout-ms",
            "8",
            "--max-body",
            "0",
            "--queue",
            "0",
            "--workers",
            "256",
            "--addr",
            "[::1]:0",
            "--root",
            ".",
        ]);
        assert_eq!(config.addr, "[::1]:0".parse().unwrap());
        assert_eq!((config.workers, config.queue, config.max_body), (256, 0, 0));
        assert_eq!(config.request_timeout, Duration::from_millis(8));
        assert_eq!(config.idle_timeout, Duration::from_millis(7));
    }

    #[test]
    fn help_wins_over_errors() {
        assert_eq!(cli(&["--help"]), Ok(Cli::Help));
        assert_eq!(cli(&["--bogus", "--help"]), Ok(Cli::Help));
    }

    #[test]
    fn mistakes_are_named() {
        assert_eq!(cli(&[]), Err(UsageError::MissingRoot));
        assert_eq!(cli(&["--workers", "2"]), Err(UsageError::MissingRoot));
        assert_eq!(cli(&["--root"]), Err(UsageError::MissingValue("--root")));
        assert_eq!(
            cli(&["--root", "a", "--root", "b"]),
            Err(UsageError::Repeated("--root"))
        );
        assert_eq!(
            cli(&["--root", "a", "extra"]),
            Err(UsageError::UnknownArgument("extra".into()))
        );
        assert_eq!(
            cli(&["--root=a"]),
            Err(UsageError::UnknownArgument("--root=a".into()))
        );
    }

    #[test]
    fn numbers_must_be_plain_digits_in_range() {
        for (option, bad) in [
            ("--workers", "0"),
            ("--workers", "257"),
            ("--workers", "+4"),
            ("--workers", "four"),
            ("--workers", ""),
            ("--queue", "-1"),
            ("--queue", "4097"),
            ("--max-body", "16777217"),
            ("--max-body", "99999999999999999999999"),
            ("--request-timeout-ms", "0"),
            ("--idle-timeout-ms", "600001"),
            ("--idle-timeout-ms", "1.5"),
        ] {
            let result = cli(&["--root", "a", option, bad]);
            assert!(
                matches!(result, Err(UsageError::BadValue { option: o, .. }) if o == option),
                "{option} {bad:?} should be a bad value, got {result:?}"
            );
        }
        assert!(matches!(
            cli(&["--root", "a", "--addr", "localhost"]),
            Err(UsageError::BadValue {
                option: "--addr",
                ..
            })
        ));
    }
}
