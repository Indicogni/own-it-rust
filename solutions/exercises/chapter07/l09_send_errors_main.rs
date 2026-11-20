//! LESSON 09 — Thread-safe boxed errors and what main returns
//! Run: cargo test --test chapter07 l09_
//! Read first: CONCEPTS.md, section "Lesson 09".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Why can a Box<dyn Error> not be returned from a spawned thread?
//! 2. When `main` returns Err, what is printed, and which exit status does
//!    the process end with?
//!
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain what `+ Send + Sync + 'static` promises about a
//! boxed error, in which direction the two boxed error types convert, and
//! when a program should return ExitCode from main instead of a Result.
//!
//! COMPILER LAB: change the `SendError` alias below to plain `Box<dyn Error>`
//! and read the E0277 diagnostic that `on_worker` now produces. Change it
//! back. Drill d10 shows the same error in a twelve-line program.
//! OPTIONAL LAB: LABS.md, lab 5 (a real `main`, stderr and the exit status).

use std::error::Error;
use std::io::{Read, Write};
use std::process::ExitCode;

/// A boxed error that may be moved to another thread (`Send`), shared between
/// threads (`Sync`), and that borrows nothing (`'static`).
type SendError = Box<dyn Error + Send + Sync + 'static>;
type SendResult<T> = Result<T, SendError>;

/// PROVIDED; do not change. Runs `job` on a new thread, waits for it and
/// returns its value. The bounds are the ones `std::thread::spawn` itself
/// has: the closure and its result must both be `Send + 'static`.
/// (Chapter 19 teaches threads; here you only need to call this function.)
fn on_worker<T, F>(job: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match std::thread::spawn(job).join() {
        Ok(value) => value,
        // The job panicked: continue the same panic on the calling thread.
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// A. Read the whole input with `Read::read_to_string`. Every line that is
/// not blank after trimming holds one u32; return the sum of the lines.
/// Stop at the first failing line. Propagate I/O and parse errors with ?.
/// If the sum does not fit in u32, fail with an error whose Display text is
/// exactly "sum overflows u32" (a &str converts into SendError).
fn sum_lines(mut reader: impl Read) -> SendResult<u32> {
    let mut text = String::new();
    reader.read_to_string(&mut text)?;
    let mut total: u32 = 0;
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let value: u32 = line.parse()?;
        // ok_or turns None into Err(&str); ? then boxes the &str.
        total = total.checked_add(value).ok_or("sum overflows u32")?;
    }
    Ok(total)
}

/// B. Do the work of task A on a worker thread: give `on_worker` a closure
/// that owns `reader` and calls `sum_lines`. The input must be read on the
/// worker; the sum, or the boxed error, travels back to the caller.
fn sum_on_worker(reader: impl Read + Send + 'static) -> SendResult<u32> {
    // `move` gives the reader to the worker. The SendResult may come back
    // because its error type is Send.
    on_worker(move || sum_lines(reader))
}

/// C. An older reporting function accepts only a plain `Box<dyn Error>`.
/// Convert the error side for it. The error must stay in the same heap
/// allocation and keep its concrete type: no new Box, no `to_string`.
/// (`?` cannot do this conversion, because no `From` impl exists in this
/// direction. Something simpler can.)
fn widen(result: SendResult<u32>) -> Result<u32, Box<dyn Error>> {
    // A cast that only forgets the Send + Sync promises: same pointer, same
    // vtable, no allocation.
    result.map_err(|error| error as Box<dyn Error>)
}

/// D. The last step of a `main` that returns `ExitCode`. The `stderr`
/// parameter stands in for the real standard error stream, so that a test
/// can read what was written.
/// - Ok(_): write nothing and return `ExitCode::SUCCESS`.
/// - Err(e): write exactly one line: "error: ", then e's Display text, then
///   '\n'. Return `ExitCode::from(2)` if the concrete error is a
///   `ParseIntError` (the input data is wrong), and `ExitCode::FAILURE` for
///   every other error.
///
/// If writing that line fails, there is nowhere left to report it: ignore
/// that one failure on purpose (lesson 07) and still return the status.
fn exit_code(outcome: &SendResult<u32>, stderr: &mut dyn Write) -> ExitCode {
    match outcome {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            // If stderr itself is broken there is nowhere to report that.
            let _ = writeln!(stderr, "error: {error}");
            if error.is::<std::num::ParseIntError>() {
                ExitCode::from(2)
            } else {
                ExitCode::FAILURE
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use practice::within;
    use std::io::{self, ErrorKind};
    use std::num::ParseIntError;
    use std::process::Termination;
    use std::sync::{Arc, Mutex};
    use std::thread::{self, ThreadId};

    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(ErrorKind::PermissionDenied, "blocked"))
        }
    }

    /// Yields `data` and records the thread on which each read happens.
    struct Tracked {
        data: &'static [u8],
        seen: Arc<Mutex<Vec<ThreadId>>>,
    }
    impl Read for Tracked {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.seen.lock().unwrap().push(thread::current().id());
            self.data.read(buffer)
        }
    }

    /// A writer that always fails, like a closed stderr.
    struct Closed;
    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(ErrorKind::BrokenPipe, "stderr is closed"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// The address of the error value that a boxed error points at.
    fn address(error: &(dyn Error + 'static)) -> usize {
        std::ptr::from_ref(error).addr()
    }

    /// Named in the failure message when a test below panics or never ends.
    const READING: &str =
        "sum_lines [if it never returns: a read loop must stop when read gives Ok(0)]";

    // Tests that read input run inside `within`, so a read loop that never
    // ends fails after a few seconds instead of hanging the suite. A
    // SendResult can be handed back to the test thread, because its error is
    // Send. Compare lesson 05, where every assertion had to stay inside.
    #[test]
    fn a_sum_lines_adds_the_non_blank_lines() {
        let sum = |text: &'static str| within(5, READING, move || sum_lines(text.as_bytes()));
        assert_eq!(sum("1\n2\n\n 3 ").unwrap(), 6);
        assert_eq!(sum("").unwrap(), 0);
        assert_eq!(sum(" \n\t\n").unwrap(), 0, "blank lines are skipped");
        assert_eq!(sum("4294967295\r\n0\r\n").unwrap(), u32::MAX);
    }

    #[test]
    fn a_three_kinds_of_failure_fit_in_one_boxed_type() {
        let parse = within(5, READING, || sum_lines("1\nx\n5".as_bytes())).unwrap_err();
        assert!(
            parse.is::<ParseIntError>(),
            "a bad line must stay a ParseIntError inside the box; got {parse:?}"
        );

        let io = within(5, READING, || sum_lines(Broken)).unwrap_err();
        let io = io
            .downcast_ref::<io::Error>()
            .expect("the reader's io::Error, boxed unchanged");
        assert_eq!(
            io.kind(),
            ErrorKind::PermissionDenied,
            "the original io::Error must be boxed, not a new one"
        );

        let overflow = within(5, READING, || sum_lines("4294967295\n1".as_bytes()));
        let overflow = overflow.unwrap_err();
        assert_eq!(
            overflow.to_string(),
            "sum overflows u32",
            "an overflowing sum fails with exactly this Display text"
        );
        assert!(
            !overflow.is::<ParseIntError>() && !overflow.is::<io::Error>(),
            "overflow of the sum is its own failure, made from the &str message"
        );
    }

    #[test]
    fn a_the_first_failing_line_wins() {
        let outcome = within(5, READING, || sum_lines("4294967295\n1\nx".as_bytes()));
        assert_eq!(
            outcome.unwrap_err().to_string(),
            "sum overflows u32",
            "line 2 overflows before line 3 is parsed: add as you go"
        );
        let outcome = within(5, READING, || sum_lines("x\n4294967295\n1".as_bytes()));
        assert!(
            outcome.unwrap_err().is::<ParseIntError>(),
            "line 1 is bad, so its ParseIntError is returned before any overflow"
        );
    }

    #[test]
    fn b_the_input_is_read_on_the_worker_thread() {
        within(5, READING, || {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let reader = Tracked {
                data: b"20\n22\n",
                seen: Arc::clone(&seen),
            };
            let caller = thread::current().id();
            assert_eq!(sum_on_worker(reader).unwrap(), 42);
            let seen = seen.lock().unwrap();
            assert!(!seen.is_empty(), "the reader was never read");
            assert!(
                seen.iter().all(|id| *id != caller),
                "the input must be read on the worker thread (use on_worker), \
                 not on the caller's thread"
            );
        });
    }

    #[test]
    fn b_errors_travel_back_from_the_worker() {
        let io = within(5, READING, || sum_on_worker(Broken)).unwrap_err();
        assert!(
            io.is::<io::Error>(),
            "the worker's io::Error must arrive on this thread unchanged; got {io:?}"
        );
        let parse = within(5, READING, || sum_on_worker("7\nseven".as_bytes()));
        assert!(
            parse.unwrap_err().is::<ParseIntError>(),
            "the worker's ParseIntError must arrive unchanged"
        );
        let overflow = within(5, READING, || sum_on_worker("4294967295\n1".as_bytes()));
        assert_eq!(
            overflow.unwrap_err().to_string(),
            "sum overflows u32",
            "the string error made on the worker must arrive unchanged"
        );
    }

    #[test]
    fn c_widen_keeps_the_same_boxed_error() {
        let error: SendError = Box::new(io::Error::new(ErrorKind::NotFound, "gone"));
        let before = address(&*error);
        let (widened, allocations) = count(|| widen(Err(error)));
        let plain = widened.unwrap_err();
        assert_eq!(
            address(&*plain),
            before,
            "widen must hand over the same heap allocation, not build a new error"
        );
        assert_eq!(
            allocations, 0,
            "dropping Send + Sync from the type is free: no new Box, no String"
        );
        let io = plain
            .downcast_ref::<io::Error>()
            .expect("the concrete error type must survive the conversion");
        assert_eq!(io.kind(), ErrorKind::NotFound);
        assert_eq!(widen(Ok(3)).ok(), Some(3), "an Ok value passes through");
    }

    #[test]
    fn d_success_is_silent_and_exits_with_zero() {
        let mut stderr = Vec::new();
        assert_eq!(
            exit_code(&Ok(7), &mut stderr),
            ExitCode::SUCCESS,
            "Ok means exit status 0"
        );
        assert!(
            stderr.is_empty(),
            "a successful run writes nothing to stderr"
        );
    }

    #[test]
    fn d_failures_choose_a_status_and_print_display() {
        let parse_error = || "x".parse::<u32>().unwrap_err();
        let mut stderr = Vec::new();
        let status = exit_code(&Err(parse_error().into()), &mut stderr);
        assert_eq!(
            status,
            ExitCode::from(2),
            "wrong input data (a ParseIntError) is status 2"
        );
        assert_eq!(
            String::from_utf8(stderr).unwrap(),
            format!("error: {}\n", parse_error()),
            "one line: `error: ` and the error's Display text"
        );

        let mut stderr = Vec::new();
        let status = exit_code(&Err("disk full".into()), &mut stderr);
        assert_eq!(
            status,
            ExitCode::FAILURE,
            "every other error is the general failure status"
        );
        assert_eq!(
            String::from_utf8(stderr).unwrap(),
            "error: disk full\n",
            "print the error with Display ({{}}), not with Debug ({{:?}})"
        );

        let mut stderr = Vec::new();
        let unreadable: SendError = Box::new(io::Error::new(ErrorKind::NotFound, "no file"));
        assert_eq!(
            exit_code(&Err(unreadable), &mut stderr),
            ExitCode::FAILURE,
            "an io::Error is not wrong input data: general failure status"
        );
        assert_eq!(String::from_utf8(stderr).unwrap(), "error: no file\n");
    }

    #[test]
    fn d_a_failed_write_to_stderr_changes_nothing() {
        let status = exit_code(&Err("x".parse::<u32>().unwrap_err().into()), &mut Closed);
        assert_eq!(
            status,
            ExitCode::from(2),
            "if stderr cannot be written, still return the status (and do not panic)"
        );
        assert_eq!(
            exit_code(&Ok(1), &mut Closed),
            ExitCode::SUCCESS,
            "success does not write, so a closed stderr cannot matter"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// The line starting with "Error:" that this test prints is expected.
    #[test]
    fn p_predictions() {
        // P1. Stack size, in machine words, of Box<dyn Error> and of
        // SendError. Do the extra bounds cost anything at run time?
        let p1: (usize, usize) = (2, 2);
        assert_eq!(
            (p1.0 * size_of::<usize>(), p1.1 * size_of::<usize>()),
            (size_of::<Box<dyn Error>>(), size_of::<SendError>()),
            "P1: what Send and Sync add to a boxed error"
        );

        // P2. A boxed error made from a &str. Write its Display text, then its
        // Debug text. (A `main` that returns Err prints the Debug text.)
        let failure = SendError::from("disk full");
        let p2: (&str, &str) = ("disk full", "\"disk full\"");
        assert_eq!(
            (p2.0.to_string(), p2.1.to_string()),
            (format!("{failure}"), format!("{failure:?}")),
            "P2: Display and Debug of a string error"
        );

        // P3. `main` may return any type that implements Termination. The
        // runtime calls report() on the returned value to get the process
        // exit status. Which status number does an Err give?
        let returned: Result<(), SendError> = Err(failure);
        let p3: u8 = 1;
        assert_eq!(
            returned.report(),
            ExitCode::from(p3),
            "P3: the exit status of a main that returns Err"
        );
    }
}
