//! The command line: the script runner and the REPL (SPEC sections 2, 12).
//!
//! Everything here works on streams passed in by the caller, so the whole
//! driver can be run inside a test without a process.

use std::ffi::OsString;
use std::io::{BufRead, Write};
use std::path::Path;

use crate::lexer;
use crate::session::{Failure, Session};
use crate::source::Source;
use crate::value::Value;

pub const EXIT_OK: u8 = 0;
pub const EXIT_RUNTIME_ERROR: u8 = 1;
pub const EXIT_COMPILE_ERROR: u8 = 2;
/// Wrong arguments, an unreadable script, or output that cannot be written.
pub const EXIT_USAGE_OR_IO: u8 = 3;

const USAGE: &str = "usage: tansy [script]";

/// Runs the interpreter with `args` (the program name not included) and
/// returns the exit status.
///
/// Errors while writing to `err` are ignored: there is no place left to
/// report them.
pub fn run(
    args: &[OsString],
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    match args {
        [] => repl(input, out, err),
        [arg] if arg == "-h" || arg == "--help" => {
            let written = writeln!(
                out,
                "{USAGE}\n\nWith a script: runs it.\nWithout: reads Tansy from standard input, one entry at a time."
            );
            match written.and_then(|()| out.flush()) {
                Ok(()) => EXIT_OK,
                Err(e) => output_failed(&e, err),
            }
        }
        [script] if !script.to_string_lossy().starts_with('-') => run_file(script, input, out, err),
        _ => {
            let _ = writeln!(err, "{USAGE}");
            EXIT_USAGE_OR_IO
        }
    }
}

fn output_failed(error: &std::io::Error, err: &mut dyn Write) -> u8 {
    let _ = writeln!(err, "error: cannot write output: {error}");
    EXIT_USAGE_OR_IO
}

fn run_file(
    script: &OsString,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    // Diagnostics show the path as it was given.
    let name = script.to_string_lossy();
    let text = match std::fs::read(Path::new(script)).map(String::from_utf8) {
        Ok(Ok(text)) => text,
        Ok(Err(_)) => {
            let _ = writeln!(
                err,
                "error: cannot read {name}: the file is not valid UTF-8"
            );
            return EXIT_USAGE_OR_IO;
        }
        Err(e) => {
            let _ = writeln!(err, "error: cannot read {name}: {e}");
            return EXIT_USAGE_OR_IO;
        }
    };
    let source = Source {
        name: &name,
        text: &text,
    };
    let mut session = Session::new(out, input);
    let result = session.run(&text);
    // Everything the script printed goes out before the error message.
    let flushed = session.out().flush();
    match (result, flushed) {
        (Err(Failure::Output(e)), _) | (_, Err(e)) => output_failed(&e, err),
        (Ok(_), Ok(())) => EXIT_OK,
        (Err(Failure::Exit(code)), Ok(())) => code,
        (Err(Failure::Compile(diagnostic)), Ok(())) => {
            let _ = err.write_all(source.render(&diagnostic).as_bytes());
            EXIT_COMPILE_ERROR
        }
        (Err(Failure::Runtime(diagnostic)), Ok(())) => {
            let _ = err.write_all(source.render(&diagnostic).as_bytes());
            EXIT_RUNTIME_ERROR
        }
    }
}

fn repl(input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let mut session = Session::new(out, input);
    // Every source line of the session, in one text. An entry is the part
    // from `entry_start` to the end. Keeping the old entries is what lets a
    // diagnostic show a line of a function that was typed earlier, and it
    // numbers the lines of the session in one sequence (SPEC 12.5).
    let mut text = String::new();
    let mut entry_start = 0;
    loop {
        let prompt = if entry_start == text.len() {
            "> "
        } else {
            ". "
        };
        // The flush is what makes the session interactive: the prompt and
        // all output so far are visible before the read blocks.
        if let Err(e) = write!(session.out(), "{prompt}").and_then(|()| session.out().flush()) {
            return output_failed(&e, err);
        }
        let line = match session.read_line() {
            Ok(line) => line,
            Err(e) => {
                let _ = writeln!(err, "error: cannot read input: {e}");
                return EXIT_USAGE_OR_IO;
            }
        };
        let at_end = line.is_none();
        if let Some(line) = line {
            text.push_str(&String::from_utf8_lossy(&line));
            text.push('\n');
            if lexer::needs_more_input(&text[entry_start..]) {
                continue;
            }
        }

        // At the end of input an unfinished entry is evaluated as it is, so
        // that its error is reported.
        let result = session.run_from(&text, entry_start);
        entry_start = text.len();
        let shown = match &result {
            Ok(Value::Nil) | Err(_) => Ok(()),
            Ok(value) => writeln!(session.out(), "{}", value.quoted()),
        };
        let flushed = shown.and_then(|()| session.out().flush());
        match (result, flushed) {
            (Err(Failure::Output(e)), _) | (_, Err(e)) => return output_failed(&e, err),
            (Err(Failure::Exit(code)), Ok(())) => return code,
            (Err(Failure::Compile(diagnostic) | Failure::Runtime(diagnostic)), Ok(())) => {
                let source = Source {
                    name: "<repl>",
                    text: &text,
                };
                let _ = err.write_all(source.render(&diagnostic).as_bytes());
            }
            (Ok(_), Ok(())) => {}
        }

        if at_end {
            // Leave the terminal on a fresh line after Ctrl-D.
            return match writeln!(session.out()).and_then(|()| session.out().flush()) {
                Ok(()) => EXIT_OK,
                Err(e) => output_failed(&e, err),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs the REPL on `input`; returns (stdout, stderr, status).
    fn repl_session(input: &str) -> (String, String, u8) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = run(&[], &mut input.as_bytes(), &mut out, &mut err);
        (
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
            status,
        )
    }

    #[test]
    fn values_are_echoed_in_quoted_form_and_nil_is_not() {
        let (out, err, status) = repl_session("1 + 2\n\"a\" + \"b\"\nnil\nlet x = [1]\nx\n");
        assert_eq!(out, "> 3\n> \"ab\"\n> > > [1]\n> \n");
        assert_eq!((err.as_str(), status), ("", 0));
    }

    #[test]
    fn an_entry_continues_while_a_delimiter_is_open() {
        let (out, err, _) = repl_session("fn twice(x) {\n  x * 2\n}\ntwice(\n21)\n");
        assert_eq!(out, "> . . > . 42\n> \n");
        assert_eq!(err, "");
    }

    #[test]
    fn errors_do_not_end_the_session() {
        let (out, err, status) = repl_session("let a = 1\na +\na / 0\na\n");
        assert_eq!(out, "> > > > 1\n> \n");
        assert_eq!(
            err,
            "<repl>:3:1: error: expected expression, found end of input\n    \n    ^\n\
             <repl>:3:1: error: division by zero\n    a / 0\n    ^^^^^\n"
        );
        assert_eq!(status, 0);
    }

    #[test]
    fn an_error_in_an_earlier_entry_shows_the_line_of_that_entry() {
        let (out, err, _) = repl_session("fn half(n) {\n  n / 0\n}\n1\nhalf(8)\n");
        assert_eq!(out, "> . . > 1\n> > \n");
        assert_eq!(
            err,
            "<repl>:2:3: error: division by zero\n      n / 0\n      ^^^^^\n"
        );
    }

    #[test]
    fn input_and_the_repl_share_the_lines() {
        let (out, _, _) = repl_session("let line = input()\nthis is data\nline\n");
        assert_eq!(out, "> > \"this is data\"\n> \n");
    }

    #[test]
    fn exit_ends_the_session_at_once() {
        let (out, err, status) = repl_session("print(1)\nexit(9)\nprint(2)\n");
        assert_eq!((out.as_str(), err.as_str(), status), ("> 1\n> ", "", 9));
    }

    #[test]
    fn an_unfinished_entry_at_the_end_of_input_is_reported() {
        let (out, err, status) = repl_session("print((1)\n");
        assert_eq!(out, "> . \n");
        assert_eq!(
            err,
            "<repl>:2:1: error: expected `,` or `)`, found end of input\n    \n    ^\n"
        );
        assert_eq!(status, 0);
    }

    #[test]
    fn wrong_arguments_print_the_usage() {
        for args in [vec!["a.tn", "b.tn"], vec!["--frobnicate"], vec!["-"]] {
            let args: Vec<OsString> = args.into_iter().map(OsString::from).collect();
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let status = run(&args, &mut "".as_bytes(), &mut out, &mut err);
            assert_eq!(
                (out.as_slice(), err.as_slice(), status),
                (&b""[..], &b"usage: tansy [script]\n"[..], 3)
            );
        }
    }
}
