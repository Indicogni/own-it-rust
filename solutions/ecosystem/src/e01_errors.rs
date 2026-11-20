//! LESSON E01 — Library errors with thiserror, application errors with anyhow (after Chapter 7)
//! Run: cargo test --manifest-path ecosystem/Cargo.toml e01_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Which layer should name every failure
//! as a variant, and which layer only needs to explain failures to a human? What does
//! `{}` print for an error that has a source chain?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: When does a function return `Result<T, MyError>` and when
//! `anyhow::Result<T>`? Why should a Display message not repeat its source's message?
//! JUDGMENT: thiserror only generates the `Display`/`Error`/`From` impls you would
//! write by hand. It adds nothing to your public API. anyhow is for binaries and glue
//! code, not for public library signatures.

use std::io::BufRead;
use std::num::ParseIntError;
use std::path::Path;
use thiserror::Error;

/// Parsed `key=value` settings for a tiny service.
#[derive(Debug, PartialEq, Eq)]
pub struct Settings {
    pub host: String,
    pub port: u16,
}

/// A. The library error. Make each variant match its documented contract:
///
/// - `MissingEquals { line }` displays as ``line 3: expected `key=value` ``.
/// - `BadPort { line, cause }` displays as `line 2: invalid port`, and
///   `Error::source()` returns the `ParseIntError`. Do not repeat the source's
///   message in the Display text; printing the chain does that.
/// - `MissingKey(&'static str)` displays as ``missing key `port` ``.
/// - `Io(std::io::Error)` displays as `could not read settings`, its source is
///   the I/O error, and `?` converts an `io::Error` into it.
#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("line {line}: expected `key=value`")]
    MissingEquals { line: usize },
    #[error("line {line}: invalid port")]
    BadPort {
        line: usize,
        #[source]
        cause: ParseIntError,
    },
    #[error("missing key `{0}`")]
    MissingKey(&'static str),
    #[error("could not read settings")]
    Io(#[from] std::io::Error),
}

/// B. Parse settings from a reader, one `key=value` per line (lines numbered from 1).
/// Trim whitespace around keys and values. Skip blank lines and lines starting with `#`.
/// Keys `host` and `port` are required; later duplicates win; unknown keys are ignored.
/// Read failures propagate with `?` as `SettingsError::Io`.
pub fn parse_settings(reader: impl BufRead) -> Result<Settings, SettingsError> {
    let mut host = None;
    let mut port = None;
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        let number = index + 1;
        let text = line.trim();
        if text.is_empty() || text.starts_with('#') {
            continue;
        }
        let (key, value) = text
            .split_once('=')
            .ok_or(SettingsError::MissingEquals { line: number })?;
        let (key, value) = (key.trim(), value.trim());
        match key {
            "host" => host = Some(value.to_owned()),
            "port" => {
                port = Some(
                    value
                        .parse::<u16>()
                        .map_err(|cause| SettingsError::BadPort {
                            line: number,
                            cause,
                        })?,
                );
            }
            _ => {}
        }
    }
    Ok(Settings {
        host: host.ok_or(SettingsError::MissingKey("host"))?,
        port: port.ok_or(SettingsError::MissingKey("port"))?,
    })
}

/// C. Application layer: open the file at `path` and parse it with `parse_settings`.
/// Add the context `opening settings file {path}` to open failures (use `Path::display`)
/// and `parsing {path}` to parse failures. Return `anyhow::Result`.
pub fn load_settings(path: &Path) -> anyhow::Result<Settings> {
    use anyhow::Context as _;
    let file = std::fs::File::open(path)
        .with_context(|| format!("opening settings file {}", path.display()))?;
    parse_settings(std::io::BufReader::new(file))
        .with_context(|| format!("parsing {}", path.display()))
}

/// D. Map an application error to a process exit code by walking its chain, outermost
/// first, and returning the code for the first recognized error:
/// `SettingsError` → 78 (EX_CONFIG), `io::Error` with kind `NotFound` → 66 (EX_NOINPUT).
/// Anything else → 1.
pub fn exit_code(err: &anyhow::Error) -> i32 {
    for cause in err.chain() {
        if cause.is::<SettingsError>() {
            return 78;
        }
        if let Some(io) = cause.downcast_ref::<std::io::Error>()
            && io.kind() == std::io::ErrorKind::NotFound
        {
            return 66;
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use std::error::Error as _;
    use std::io::{self, Cursor, Read};

    fn parse(text: &str) -> Result<Settings, SettingsError> {
        parse_settings(Cursor::new(text))
    }

    /// A reader that fails on its first read, like a disk error halfway through a file.
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("disk on fire"))
        }
    }

    #[test]
    fn a_messages_follow_the_contract() {
        assert_eq!(
            SettingsError::MissingEquals { line: 3 }.to_string(),
            "line 3: expected `key=value`"
        );
        assert_eq!(
            SettingsError::MissingKey("port").to_string(),
            "missing key `port`"
        );
        let io = SettingsError::Io(io::Error::other("boom"));
        assert_eq!(io.to_string(), "could not read settings");
        assert_eq!(io.source().unwrap().to_string(), "boom");
    }

    #[test]
    fn a_bad_port_keeps_its_source_without_repeating_it() {
        let err = parse("host=a\nport=99999\n").unwrap_err();
        assert!(matches!(err, SettingsError::BadPort { line: 2, .. }));
        assert_eq!(err.to_string(), "line 2: invalid port");
        let source = err.source().expect("BadPort must expose its cause");
        assert!(source.downcast_ref::<ParseIntError>().is_some());
    }

    #[test]
    fn b_parses_trims_and_skips() {
        let text = "# service\n\n host = example.org \nport= 8080\nmode=fast\nport=9090\n";
        assert_eq!(
            parse(text).unwrap(),
            Settings {
                host: "example.org".into(),
                port: 9090
            }
        );
    }

    #[test]
    fn b_reports_the_first_problem() {
        assert!(matches!(
            parse("host=a\nport\n"),
            Err(SettingsError::MissingEquals { line: 2 })
        ));
        assert!(matches!(
            parse("port=1\n"),
            Err(SettingsError::MissingKey("host"))
        ));
        assert!(matches!(
            parse("host=a"),
            Err(SettingsError::MissingKey("port"))
        ));
        assert!(matches!(
            parse_settings(io::BufReader::new(Broken)),
            Err(SettingsError::Io(_))
        ));
    }

    #[test]
    fn c_context_wraps_without_hiding_the_cause() {
        let dir = TempDir::new("e01-c");
        let missing = dir.0.join("does-not-exist.conf");
        let err = load_settings(&missing).unwrap_err();
        let text = format!("{err:#}");
        assert!(
            text.starts_with(&format!("opening settings file {}: ", missing.display())),
            "{text}"
        );
        let io = err.downcast_ref::<io::Error>().expect("still an io::Error");
        assert_eq!(io.kind(), io::ErrorKind::NotFound);

        let path = dir.file("bad.conf", "host=a\nport=x\n");
        let err = load_settings(&path).unwrap_err();
        assert_eq!(err.to_string(), format!("parsing {}", path.display()));
        assert!(matches!(
            err.downcast_ref::<SettingsError>(),
            Some(SettingsError::BadPort { line: 2, .. })
        ));
        assert_eq!(err.chain().count(), 3, "context → BadPort → ParseIntError");

        let path = dir.file("good.conf", "host=h\nport=1\n");
        assert_eq!(load_settings(&path).unwrap().port, 1);
    }

    #[test]
    fn d_exit_codes_come_from_the_chain() {
        let dir = TempDir::new("e01-d");
        let missing = dir.0.join("missing-too.conf");
        assert_eq!(exit_code(&load_settings(&missing).unwrap_err()), 66);
        let path = dir.file("noport.conf", "host=h\n");
        assert_eq!(exit_code(&load_settings(&path).unwrap_err()), 78);
        assert_eq!(exit_code(&anyhow::anyhow!("something else")), 1);
        // The outermost recognized error wins, even when an io::Error is deeper.
        let nested = anyhow::Error::new(SettingsError::Io(io::ErrorKind::NotFound.into()))
            .context("startup");
        assert_eq!(exit_code(&nested), 78);
        let permission = anyhow::Error::new(io::Error::from(io::ErrorKind::PermissionDenied));
        assert_eq!(exit_code(&permission), 1);
    }

    /// P. Predict BEFORE running: replace each `crate::guess()` call with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in your notes.
    #[test]
    fn p_predictions() {
        let err = anyhow::anyhow!("connection refused")
            .context("fetching config")
            .context("starting service");

        // P1. What does plain `{}` print for an anyhow error with context?
        let p1: &str = "starting service";
        assert_eq!(
            p1,
            format!("{err}"),
            "P1: Display shows the outermost layer"
        );

        // P2. And the alternate form `{:#}`?
        let p2: &str = "starting service: fetching config: connection refused";
        assert_eq!(
            p2,
            format!("{err:#}"),
            "P2: alternate Display joins the chain"
        );

        // P3. How many errors does `chain()` yield?
        let p3: usize = 3;
        assert_eq!(p3, err.chain().count(), "P3: each context is a layer");

        // P4. Can you downcast to the type of a context value (here `&str`)?
        let p4: bool = true;
        assert_eq!(
            p4,
            err.downcast_ref::<&str>().is_some(),
            "P4: context values are downcastable"
        );
    }
}
