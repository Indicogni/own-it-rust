//! LESSON 09 — Child processes and pipes
//! Run: cargo test --test chapter18 l09_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. A child exits with code 3. Is that an `Err` from `Command::output`?
//! 2. A parent writes 2 MiB to a child's stdin and only then starts to read the
//!    child's stdout. What can go wrong?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Why is a failed child not an io::Error? Who must close which end of
//! a pipe before a reader sees the end of the stream, and what happens when a
//! pipe is full?
//! COMPILER LAB (examples/ch18_scratch.rs): Run `rustc --version` with
//! Command::output and print its status, stdout, and stderr. Then run
//! `rustc --no-such-flag` and compare. Then start a program name that does not
//! exist and look at the error kind.

use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::thread;

/// Why a command did not produce a text result.
#[derive(Debug)]
enum RunError {
    /// The program could not be started, or waiting for it failed.
    Io(io::Error),
    /// The program ran and reported failure. `code` is None when a signal
    /// ended it (Unix only).
    Failed { code: Option<i32>, stderr: String },
    /// The program succeeded, but its standard output was not UTF-8.
    NotText,
}

/// A. Run `cmd` to completion with no input and return everything it wrote to
/// standard output, as text.
/// - Starting or waiting fails: `RunError::Io`.
/// - The exit status is not success: `RunError::Failed` with the exit code and
///   the child's standard error, converted lossily (invalid UTF-8 becomes
///   U+FFFD) and without trailing whitespace.
/// - Otherwise the output must be valid UTF-8, else `RunError::NotText`.
///
/// The child may write far more than a pipe can hold.
fn run_text(mut cmd: Command) -> Result<String, RunError> {
    // `output` gives the child no input, and it reads both pipes while it
    // waits, so a child with a lot of output cannot block on a full pipe.
    let output = cmd.output().map_err(RunError::Io)?;
    if !output.status.success() {
        return Err(RunError::Failed {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr)
                .trim_end()
                .to_string(),
        });
    }
    String::from_utf8(output.stdout).map_err(|_| RunError::NotText)
}

/// B. Start `cmd`, send it all of `input` on its standard input, and collect
/// its standard output. Return the exit code and the output. Leave the
/// child's standard error alone.
/// - Close the child's stdin after the last byte, so the child sees the end
///   of its input.
/// - `input` and the output may both be far larger than a pipe can hold, so
///   write on a second thread (`std::thread::scope`) while this thread reads.
/// - A child may exit before it has read everything. The write then fails
///   with BrokenPipe. That is the child's decision and not an error here;
///   propagate every other error.
/// - Always wait for the child.
fn filter_through(mut cmd: Command, input: &[u8]) -> io::Result<(Option<i32>, Vec<u8>)> {
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
    let mut stdin = child.stdin.take().expect("stdin was requested as a pipe");
    let mut stdout = child.stdout.take().expect("stdout was requested as a pipe");
    let mut output = Vec::new();
    let (written, read) = thread::scope(|scope| {
        let writer = scope.spawn(move || {
            let result = stdin.write_all(input);
            // Dropping the handle closes the pipe: the child sees the end
            // of its input. `move` brought the handle into this thread, so
            // it would also be dropped at the end of the closure.
            drop(stdin);
            result
        });
        // Read on this thread while the other one writes, so neither pipe
        // stays full.
        let read = stdout.read_to_end(&mut output);
        (
            writer.join().expect("the writer thread does not panic"),
            read,
        )
    });
    // Wait in every case, so the child does not stay behind as a zombie.
    let status = child.wait()?;
    read?;
    match written {
        Err(e) if e.kind() != io::ErrorKind::BrokenPipe => return Err(e),
        // Ok, or the child stopped reading early: its own decision.
        _ => {}
    }
    Ok((status.code(), output))
}

/// C. Run the pipeline `left | right`: connect left's standard output
/// directly to right's standard input, give left no input, and collect
/// right's standard output. Wait for both children. Return left's exit code,
/// right's exit code, and right's output.
///
/// `right` may exit without reading its input. `left` must then see a closed
/// pipe instead of blocking forever, so this process must not keep any copy
/// of the pipe's ends open.
fn pipeline(
    mut left: Command,
    mut right: Command,
) -> io::Result<(Option<i32>, Option<i32>, Vec<u8>)> {
    let mut first = left.stdin(Stdio::null()).stdout(Stdio::piped()).spawn()?;
    let between = first.stdout.take().expect("stdout was requested as a pipe");
    // Converting the handle gives the reading end to `right`.
    let second = right
        .stdin(Stdio::from(between))
        .stdout(Stdio::piped())
        .spawn();
    // A Command keeps the handles it was given until it is dropped. While
    // this process holds the reading end, `left` can never see a closed
    // pipe, so drop it now, before any waiting.
    drop(right);
    let second = match second {
        Ok(child) => child,
        Err(e) => {
            // Do not leave the first child behind.
            first.wait()?;
            return Err(e);
        }
    };
    // Reads right's output to the end, then waits for it.
    let output = second.wait_with_output()?;
    let first_status = first.wait()?;
    Ok((first_status.code(), output.status.code(), output.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::within;

    // The child process in these tests is this test program itself, started
    // again with an environment variable that selects a small job (see
    // `child_mode_helper` at the bottom). That works on every platform and
    // needs no other program.
    //
    // One detail: before the helper runs, the test harness in the child
    // prints its own short banner ("running 1 test") to standard output. So
    // the tests check what the output ENDS with. Standard error has no banner.
    const MODE: &str = "CH18_CHILD_MODE";
    const HELPER: &str = "l09_processes::tests::child_mode_helper";
    const MIB: usize = 1 << 20;

    // Every test runs through `within`: a parent and a child that wait for
    // each other fail with a message instead of hanging the suite.
    const RUN: &str = "run_text (a child blocks when the pipe it writes to is full: \
                       did you wait for the child before reading its output?)";
    const FILTER: &str = "filter_through (did you close the child's stdin? Do you \
                          write and read at the same time?)";
    const PIPELINE: &str = "pipeline (is every copy of the pipe's ends in this process \
                            closed, and do you read right's output before waiting?)";

    /// A command that runs this test program again as the helper for `mode`.
    fn helper(mode: &str) -> Command {
        let exe = std::env::current_exe().expect("the test program knows its own path");
        let mut cmd = Command::new(exe);
        cmd.args(["--exact", HELPER, "--nocapture"]).env(MODE, mode);
        cmd
    }

    /// 2 MiB of lower-case letters in a fixed pattern.
    fn big_input() -> Vec<u8> {
        (0..2 * MIB).map(|i| b'a' + (i % 23) as u8).collect()
    }

    #[test]
    fn a_success_returns_standard_output_as_text() {
        let text = within(30, RUN, || run_text(helper("say"))).expect("the helper exits with 0");
        assert!(
            text.ends_with("out-text"),
            "standard output should end with what the child printed, got {text:?}"
        );
        assert!(
            !text.contains("err-text"),
            "standard error is a separate stream and is not part of the result"
        );
    }
    #[test]
    fn a_failure_carries_the_exit_code_and_standard_error() {
        let result = within(30, RUN, || run_text(helper("fail")));
        match result {
            Err(RunError::Failed { code, stderr }) => {
                assert_eq!(code, Some(3), "the helper exits with code 3");
                assert_eq!(
                    stderr, "no such record \u{fffd}",
                    "stderr: lossy conversion of the byte 0xFF, trailing whitespace removed"
                );
            }
            other => panic!("a child that exits with 3 must give RunError::Failed, got {other:?}"),
        }
    }
    #[test]
    fn a_output_that_is_not_utf8_is_not_text() {
        let result = within(30, RUN, || run_text(helper("binary")));
        assert!(
            matches!(result, Err(RunError::NotText)),
            "the helper printed the bytes FF FE and exited with 0; got {result:?}"
        );
    }
    #[test]
    fn a_program_that_cannot_start_is_an_io_error() {
        let result = within(30, RUN, || run_text(Command::new("no-such-program-own-it")));
        match result {
            Err(RunError::Io(e)) => assert_eq!(
                e.kind(),
                io::ErrorKind::NotFound,
                "the operating system could not find the program"
            ),
            other => panic!("a program that does not exist must give RunError::Io, got {other:?}"),
        }
    }
    #[test]
    fn a_large_output_does_not_block_the_child() {
        let text = within(30, RUN, || run_text(helper("flood"))).expect("the helper exits with 0");
        assert!(
            text.len() >= 2 * MIB && text.ends_with(&"x".repeat(1000)),
            "all 2 MiB of output must arrive; got {} bytes",
            text.len()
        );
    }

    #[test]
    fn b_input_goes_in_and_output_comes_back() {
        let (code, out) = within(30, FILTER, || {
            filter_through(helper("upper"), b"abc \xff def")
        })
        .expect("filter_through returned an error");
        assert_eq!(code, Some(0));
        assert!(
            out.ends_with(b"ABC \xff DEF"),
            "the helper upper-cases its input; output was {:?}",
            String::from_utf8_lossy(&out)
        );
        let (code, _) = within(30, FILTER, || filter_through(helper("upper"), b""))
            .expect("an empty input is fine");
        assert_eq!(
            code,
            Some(0),
            "the child must still see the end of its input"
        );
    }
    #[test]
    fn b_two_mebibytes_flow_in_both_directions_at_once() {
        let (code, out, expected) = within(30, FILTER, || {
            let input = big_input();
            let (code, out) =
                filter_through(helper("upper"), &input).expect("filter_through returned an error");
            (code, out, input.to_ascii_uppercase())
        });
        assert_eq!(code, Some(0));
        assert!(
            out.ends_with(&expected),
            "all 2 MiB must come back upper-cased; got {} bytes",
            out.len()
        );
    }
    #[test]
    fn b_a_child_that_stops_reading_is_not_an_error() {
        // This helper exits with 5 at once and never reads its input.
        let result = within(30, FILTER, || filter_through(helper("exit5"), &big_input()));
        let (code, _) = result.expect("BrokenPipe while writing must not become the result");
        assert_eq!(code, Some(5), "the child's exit code is still reported");
        // This helper reads 4 bytes, prints them upper-cased, and exits with 0.
        let result = within(30, FILTER, || filter_through(helper("head"), &big_input()));
        let (code, out) = result.expect("BrokenPipe while writing must not become the result");
        assert_eq!(code, Some(0));
        assert!(
            out.ends_with(b"ABCD"),
            "the output the child produced before it left must be returned"
        );
    }

    #[test]
    fn c_output_of_left_becomes_input_of_right() {
        let (left, right, out) = within(30, PIPELINE, || {
            let mut say = helper("say");
            say.stderr(Stdio::null()); // keep its err-text out of the test log
            pipeline(say, helper("upper"))
        })
        .expect("pipeline returned an error");
        assert_eq!((left, right), (Some(0), Some(0)));
        assert!(
            out.ends_with(b"OUT-TEXT"),
            "left prints out-text and right upper-cases it; output was {:?}",
            String::from_utf8_lossy(&out)
        );
    }
    #[test]
    fn c_both_exit_codes_are_reported() {
        let (left, right, _) = within(30, PIPELINE, || pipeline(helper("exit5"), helper("upper")))
            .expect("pipeline returned an error");
        assert_eq!(
            (left, right),
            (Some(5), Some(0)),
            "left failed with 5; right still ran to its end"
        );
    }
    #[test]
    fn c_two_mebibytes_pass_through_the_pipe() {
        let (left, right, out) =
            within(30, PIPELINE, || pipeline(helper("flood"), helper("upper")))
                .expect("pipeline returned an error");
        assert_eq!((left, right), (Some(0), Some(0)));
        assert!(
            out.len() >= 2 * MIB && out.ends_with(&vec![b'X'; 2 * MIB]),
            "all 2 MiB must arrive upper-cased; got {} bytes",
            out.len()
        );
    }
    #[test]
    fn c_left_sees_a_closed_pipe_when_right_leaves_early() {
        // Left tries to write 2 MiB and exits with 7 when a write fails.
        // Right reads only the first 4 bytes and then exits with 0.
        let (left, right, _) = within(30, PIPELINE, || pipeline(helper("flood"), helper("head")))
            .expect("pipeline returned an error");
        assert_eq!(right, Some(0));
        assert_eq!(
            left,
            Some(7),
            "left must fail with a broken pipe once nobody can read its output"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        let (status, color, parent_has_color, start_error, stdin_bytes) =
            within(30, "the child processes of p_predictions", || {
                // P1. This helper calls std::process::exit(5).
                let status = helper("exit5").stdout(Stdio::null()).status().unwrap();

                // P2. This helper prints the value of CH18_COLOR to its
                // standard error.
                let colored = helper("env").env("CH18_COLOR", "blue").output().unwrap();
                let color = String::from_utf8(colored.stderr).unwrap();
                let parent_has_color = std::env::var_os("CH18_COLOR").is_some();

                // P3. No program has this name.
                let start_error = Command::new("no-such-program-own-it")
                    .spawn()
                    .unwrap_err()
                    .kind();

                // P4. This helper reads its standard input to the end and
                // prints the number of bytes to its standard error.
                let counted = helper("count").output().unwrap();
                let stdin_bytes = String::from_utf8(counted.stderr).unwrap();

                (
                    (status.success(), status.code()),
                    color,
                    parent_has_color,
                    start_error,
                    stdin_bytes,
                )
            });

        // P1. success() and code() for a child that exits with 5.
        let p1: (bool, Option<i32>) = (false, Some(5));
        assert_eq!(p1, status, "P1: an exit code is data, not an error");

        // P2. What the child printed, and whether THIS process has the
        // variable afterwards.
        let p2: (&str, bool) = ("blue", false);
        assert_eq!(
            p2,
            (color.as_str(), parent_has_color),
            "P2: env() changes the child's environment only"
        );

        // P3. The error kind from spawn.
        let p3: io::ErrorKind = io::ErrorKind::NotFound;
        assert_eq!(p3, start_error, "P3: starting a missing program");

        // P4. The number the child printed: how many bytes of input does a
        // child see when it is run with output()?
        let p4: &str = "0";
        assert_eq!(p4, stdin_bytes, "P4: output() gives the child no input");
    }

    /// Not a test of your code. When a test above starts this program again
    /// with CH18_CHILD_MODE set, this function is the child's whole job. In a
    /// normal test run the variable is not set and the function does nothing.
    #[test]
    fn child_mode_helper() {
        let Ok(mode) = std::env::var(MODE) else {
            return;
        };
        let mut stdin = io::stdin().lock();
        let mut stdout = io::stdout().lock();
        let mut stderr = io::stderr().lock();
        let code = match mode.as_str() {
            // Print one text to each stream.
            "say" => {
                stdout.write_all(b"out-text").unwrap();
                stderr.write_all(b"err-text").unwrap();
                0
            }
            // Complain on standard error (with a byte that is not UTF-8 and
            // trailing whitespace) and fail.
            "fail" => {
                stderr.write_all(b"no such record \xff \n").unwrap();
                3
            }
            // Succeed with output that is not UTF-8.
            "binary" => {
                stdout.write_all(b"\xff\xfe").unwrap();
                0
            }
            // Leave at once without reading anything.
            "exit5" => 5,
            // Print the value of one environment variable.
            "env" => {
                let color = std::env::var("CH18_COLOR").unwrap_or_else(|_| "unset".into());
                stderr.write_all(color.as_bytes()).unwrap();
                0
            }
            // Count the input.
            "count" => {
                let mut input = Vec::new();
                stdin.read_to_end(&mut input).unwrap();
                write!(stderr, "{}", input.len()).unwrap();
                0
            }
            // Copy input to output in upper case, piece by piece.
            "upper" => {
                let mut buf = [0u8; 8192];
                loop {
                    let n = stdin.read(&mut buf).unwrap();
                    if n == 0 {
                        break 0;
                    }
                    buf[..n].make_ascii_uppercase();
                    stdout.write_all(&buf[..n]).unwrap();
                }
            }
            // Read four bytes, print them in upper case, and leave.
            "head" => {
                let mut first = [0u8; 4];
                stdin.read_exact(&mut first).unwrap();
                first.make_ascii_uppercase();
                stdout.write_all(&first).unwrap();
                0
            }
            // Write 2 MiB; exit with 7 if the reader has gone away.
            "flood" => {
                let chunk = [b'x'; 8192];
                let mut code = 0;
                for _ in 0..(2 * MIB / chunk.len()) {
                    if stdout.write_all(&chunk).is_err() {
                        code = 7;
                        break;
                    }
                }
                code
            }
            other => panic!("unknown helper mode {other:?}"),
        };
        if stdout.flush().is_err() {
            std::process::exit(7);
        }
        std::process::exit(code);
    }
}
