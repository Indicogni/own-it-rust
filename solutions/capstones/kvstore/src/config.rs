//! The command line (SPEC 3).

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

/// One line that names every option, printed after a usage error.
pub const USAGE: &str = "usage: kvstore --dir <DIR> [--port <PORT>] [--fsync always|never] \
                         [--auto-compact <BYTES>] [--max-clients <N>]";

const DEFAULT_PORT: u16 = 7878;
const DEFAULT_AUTO_COMPACT: u64 = 1 << 20;
const DEFAULT_MAX_CLIENTS: usize = 256;

/// The settings of one server run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub dir: PathBuf,
    /// 0 lets the operating system choose a free port.
    pub port: u16,
    pub fsync: bool,
    /// 0 switches automatic compaction off.
    pub auto_compact: u64,
    pub max_clients: usize,
}

/// A command line that SPEC 3.2 does not allow. Exit code 2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageError(String);

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for UsageError {}

/// A decimal number: digits only, so that `+5` and an empty string are
/// errors although `str::parse` would accept the first.
fn number<T: std::str::FromStr>(
    option: &str,
    text: &OsString,
    range: &str,
) -> Result<T, UsageError> {
    text.to_str()
        .filter(|t| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|t| t.parse().ok())
        .ok_or_else(|| {
            UsageError(format!(
                "{option} needs a number {range}, not `{}`",
                text.display()
            ))
        })
}

/// Stores the value of an option, or fails if the option was given before.
fn once<T>(slot: &mut Option<T>, option: &str, value: T) -> Result<(), UsageError> {
    match slot.replace(value) {
        None => Ok(()),
        Some(_) => Err(UsageError(format!("{option} was given twice"))),
    }
}

impl Config {
    /// Parses the arguments after the program name. Nothing is touched on
    /// disk here: a usage error must be reported before any directory is
    /// created.
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Config, UsageError> {
        let (mut dir, mut port, mut fsync, mut auto_compact, mut max_clients) =
            (None, None, None, None, None);
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            // Every option takes exactly one value: the next argument.
            let mut value = |option: &str| {
                args.next()
                    .ok_or_else(|| UsageError(format!("{option} needs a value")))
            };
            match arg.to_str() {
                Some(option @ "--dir") => once(&mut dir, option, PathBuf::from(value(option)?))?,
                Some(option @ "--port") => {
                    let number = number(option, &value(option)?, "from 0 to 65535")?;
                    once(&mut port, option, number)?;
                }
                Some(option @ "--fsync") => {
                    let value = value(option)?;
                    let always = match value.to_str() {
                        Some("always") => true,
                        Some("never") => false,
                        _ => {
                            return Err(UsageError(format!(
                                "--fsync takes `always` or `never`, not `{}`",
                                value.display()
                            )));
                        }
                    };
                    once(&mut fsync, option, always)?;
                }
                Some(option @ "--auto-compact") => {
                    let number = number(option, &value(option)?, "of bytes")?;
                    once(&mut auto_compact, option, number)?;
                }
                Some(option @ "--max-clients") => {
                    let n: usize = number(option, &value(option)?, "of at least 1")?;
                    if n == 0 {
                        return Err(UsageError("--max-clients must be at least 1".to_string()));
                    }
                    once(&mut max_clients, option, n)?;
                }
                _ => {
                    return Err(UsageError(format!("unknown argument `{}`", arg.display())));
                }
            }
        }
        Ok(Config {
            dir: dir.ok_or_else(|| UsageError("--dir is required".to_string()))?,
            port: port.unwrap_or(DEFAULT_PORT),
            fsync: fsync.unwrap_or(true),
            auto_compact: auto_compact.unwrap_or(DEFAULT_AUTO_COMPACT),
            max_clients: max_clients.unwrap_or(DEFAULT_MAX_CLIENTS),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Config, UsageError> {
        Config::parse(args.iter().map(OsString::from))
    }

    #[test]
    fn defaults() {
        assert_eq!(
            parse(&["--dir", "data"]),
            Ok(Config {
                dir: PathBuf::from("data"),
                port: 7878,
                fsync: true,
                auto_compact: 1_048_576,
                max_clients: 256,
            })
        );
    }

    #[test]
    fn every_option_in_any_order() {
        let config = parse(&[
            "--max-clients",
            "2",
            "--auto-compact",
            "0",
            "--fsync",
            "never",
            "--port",
            "0",
            "--dir",
            "d",
        ])
        .unwrap();
        assert_eq!(
            config,
            Config {
                dir: PathBuf::from("d"),
                port: 0,
                fsync: false,
                auto_compact: 0,
                max_clients: 2
            }
        );
    }

    #[test]
    fn usage_errors() {
        let bad: &[&[&str]] = &[
            &[],
            &["--port", "0"],
            &["--dir"],
            &["--dir", "d", "--port"],
            &["--dir", "d", "--port", "http"],
            &["--dir", "d", "--port", "65536"],
            &["--dir", "d", "--port", "-1"],
            &["--dir", "d", "--port", "+80"],
            &["--dir", "d", "--port", ""],
            &["--dir", "d", "--fsync", "sometimes"],
            &["--dir", "d", "--auto-compact", "lots"],
            &["--dir", "d", "--max-clients", "0"],
            &["--dir", "d", "--verbose"],
            &["--dir", "d", "extra"],
            &["--dir", "d", "--dir", "d"],
            &["--dir", "d", "--fsync", "never", "--fsync", "never"],
        ];
        for args in bad {
            assert!(parse(args).is_err(), "{args:?} was accepted");
        }
    }
}
