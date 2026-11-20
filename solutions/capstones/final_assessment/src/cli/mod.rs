//! The command-line program, from arguments to exit status.
//!
//! ```text
//! args --> Config --> Regex
//!            |
//!            +--> walk::plan --> [Input, Input, ...]      (output order)
//!                                    |  ordered::for_each_ordered
//!                     worker threads: search::search_input --> Outcome
//!                                    |  in input order
//!                     this thread: stdout, stderr, exit status
//! ```

mod args;
mod ordered;
mod search;
mod walk;

use std::ffi::OsString;
use std::io::{self, BufWriter, Write};
use std::ops::ControlFlow;
use std::process::ExitCode;
use std::thread;

use args::{Command, Config};
use rgx::Regex;
use search::Settings;

/// SPEC 7.6.
const SELECTED: u8 = 0;
const NOTHING_SELECTED: u8 = 1;
const FAILED: u8 = 2;

/// Runs the program on the arguments after the program name.
pub fn run(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let config = match args::parse(args) {
        Ok(Command::Search(config)) => config,
        Ok(Command::Help) => {
            // If nobody reads the help text, that is not an error.
            let _ = io::stdout().write_all(args::HELP.as_bytes());
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            report(&format!("rgx: usage error: {error}\n"));
            return ExitCode::from(FAILED);
        }
    };
    let regex = match Regex::builder()
        .case_insensitive(config.ignore_case)
        .build(&config.pattern)
    {
        Ok(regex) => regex,
        Err(error) => {
            report(&pattern_error(&config.pattern, &error));
            return ExitCode::from(FAILED);
        }
    };
    ExitCode::from(search(&config, &regex))
}

/// Writes complete lines to standard error.
///
/// Not `eprintln!`: it panics when the write fails, for example when
/// standard error is a pipe whose reader has gone away. Then there is no one
/// left to tell, and the exit status is all the program can still get
/// right, so a failure here is ignored on purpose.
fn report(lines: &str) {
    let _ = io::stderr().lock().write_all(lines.as_bytes());
}

/// The three lines of SPEC 7.5 for a pattern that does not compile.
fn pattern_error(pattern: &str, error: &rgx::Error) -> String {
    // The caret goes under the character at the error's offset. Counting
    // characters, not bytes, keeps it in place after `é` or `日`.
    let column = pattern[..error.offset()].chars().count();
    format!(
        "rgx: pattern error at byte {offset}: {error}\n    {pattern}\n    {caret:>width$}\n",
        offset = error.offset(),
        caret = '^',
        width = column + 1,
    )
}

/// Searches every input and returns the exit status.
fn search(config: &Config, regex: &Regex) -> u8 {
    let plan = walk::plan(&config.paths);
    let settings = Settings {
        mode: config.mode,
        invert: config.invert,
        line_numbers: config.line_numbers,
        show_names: plan.show_names,
    };
    let threads = match config.threads {
        Some(threads) => threads.get(),
        // An unknown number of processors is one processor.
        None => thread::available_parallelism().map_or(1, |n| n.get()),
    };

    // One lock and one large buffer for the whole run: the workers never
    // touch standard output, so nobody waits for this lock.
    let mut stdout = BufWriter::with_capacity(64 * 1024, io::stdout().lock());
    let mut selected = false;
    let mut failed = false;
    let mut write_error = None;
    ordered::for_each_ordered(
        &plan.inputs,
        threads,
        || regex.matcher(),
        |matcher, input| search::search_input(input, settings, matcher),
        |outcome| {
            selected |= outcome.selected;
            if let Some(message) = outcome.error {
                failed = true;
                report(&format!("rgx: {message}\n"));
            }
            match stdout.write_all(&outcome.output) {
                Ok(()) => ControlFlow::Continue(()),
                Err(error) => {
                    write_error = Some(error);
                    ControlFlow::Break(())
                }
            }
        },
    );
    let write_error = write_error.or_else(|| stdout.flush().err());
    // `BufWriter` flushes again when it is dropped and ignores the result,
    // which is what we want after a failed write: there is no one to tell.
    drop(stdout);

    match write_error {
        // The reader has gone away, as in `rgx x big.txt | head -1`. That
        // is how pipelines end, not a failure: stop without a message.
        Some(error) if error.kind() == io::ErrorKind::BrokenPipe => {}
        Some(error) => {
            failed = true;
            report(&format!("rgx: cannot write to standard output: {error}\n"));
        }
        None => {}
    }
    match (failed, selected) {
        (true, _) => FAILED,
        (false, true) => SELECTED,
        (false, false) => NOTHING_SELECTED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_caret_counts_characters_not_bytes() {
        let error = Regex::new("日本(語").unwrap_err();
        assert_eq!(error.offset(), 6);
        assert_eq!(
            pattern_error("日本(語", &error),
            "rgx: pattern error at byte 6: missing closing parenthesis\n    日本(語\n      ^\n"
        );
        let at_start = Regex::new("*a").unwrap_err();
        assert_eq!(
            pattern_error("*a", &at_start),
            "rgx: pattern error at byte 0: nothing to repeat\n    *a\n    ^\n"
        );
    }
}
