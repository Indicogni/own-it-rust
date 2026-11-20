//! Helpers shared by the acceptance tests.
//!
//! Every test runs the `tansy` binary that Cargo built for this package, as
//! a separate process, and looks only at what a user could see: standard
//! output, standard error and the exit status. Nothing here knows how your
//! interpreter is built.
//!
//! Each run
//!
//! - gets its own empty directory under the system's temporary directory,
//!   which is removed afterwards, also when the test fails;
//! - is stopped after [`TIMEOUT`] (the test then fails with what was printed
//!   so far), so a wrong interpreter cannot hang the test suite;
//! - is stopped when it has printed more than [`OUTPUT_LIMIT`] bytes, so a
//!   loop that prints for ever cannot fill the memory of the test;
//! - never leaves a process behind: the child is killed if the test panics.
//!
//! Each file in `tests/` is its own crate and uses only some of the helpers,
//! so unused-code warnings are switched off here.
#![allow(dead_code)]

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How long one run of the interpreter may take. Ordinary runs take a few
/// milliseconds; the limit only has to be long enough for a busy machine.
pub const TIMEOUT: Duration = Duration::from_secs(20);

/// The limit for the few tests that do real work (see `m7_programs.rs`).
pub const LONG_TIMEOUT: Duration = Duration::from_secs(120);

/// How much one run may write to standard output or to standard error. The
/// largest output that a test expects is under 3 MiB.
pub const OUTPUT_LIMIT: usize = 32 * 1024 * 1024;

/// The path of the binary under test.
pub const TANSY: &str = env!("CARGO_BIN_EXE_tansy");

/// The exit statuses of SPEC 2.2.
pub const OK: i32 = 0;
pub const RUNTIME_ERROR: i32 = 1;
pub const COMPILE_ERROR: i32 = 2;
pub const USAGE_OR_IO_ERROR: i32 = 3;

// ---------------------------------------------------------------------------
// Temporary directories
// ---------------------------------------------------------------------------

/// A new, empty directory that is deleted when the value is dropped.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> TempDir {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let base = std::env::temp_dir();
        loop {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.subsec_nanos());
            let name = format!(
                "tansy-test-{}-{}-{nanos:09}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            );
            let path = base.join(name);
            match fs::create_dir(&path) {
                Ok(()) => return TempDir(path),
                // Somebody else has this name: take the next one.
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!(
                    "cannot create a temporary directory in {}: {e}",
                    base.display()
                ),
            }
        }
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Writes `contents` to the file `name` (which may contain `/` for a
    /// sub-directory) inside the directory.
    pub fn write(&self, name: &str, contents: impl AsRef<[u8]>) {
        let path = self.0.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("cannot create {}: {e}", parent.display()));
        }
        fs::write(&path, contents)
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// ---------------------------------------------------------------------------
// Running the interpreter to the end
// ---------------------------------------------------------------------------

/// Kills and reaps the child process when dropped, so that a failing test
/// never leaves an interpreter running.
struct Reaper(Child);

impl Drop for Reaper {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// What one finished run of the interpreter produced.
pub struct Run {
    pub stdout: String,
    pub stderr: String,
    /// The exit status, or `None` if the process was ended by a signal
    /// (on Unix that is what a crash looks like).
    pub code: Option<i32>,
    /// What was run, for failure messages.
    what: String,
}

fn spawn(dir: &Path, args: &[&str]) -> Child {
    Command::new(TANSY)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot start {TANSY}: {e}"))
}

/// One output stream of the child, read on a thread of its own.
struct Capture {
    bytes: JoinHandle<Vec<u8>>,
    /// Set when the child wrote more than [`OUTPUT_LIMIT`] bytes.
    flooded: Arc<AtomicBool>,
}

impl Capture {
    fn flooded(&self) -> bool {
        self.flooded.load(Ordering::Relaxed)
    }

    /// Waits for the end of the stream (the child has ended or was killed)
    /// and returns what was read, as text.
    fn finish(self) -> String {
        String::from_utf8_lossy(&self.bytes.join().expect("reader thread")).into_owned()
    }
}

/// Reads `stream` to its end, and keeps at most [`OUTPUT_LIMIT`] bytes.
fn read_all(mut stream: impl Read + Send + 'static) -> Capture {
    let flooded = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&flooded);
    let bytes = thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => return bytes,
                // Past the limit: go on reading, so that the child is not
                // blocked, but keep nothing. The test stops the child.
                Ok(_) if bytes.len() > OUTPUT_LIMIT => flag.store(true, Ordering::Relaxed),
                Ok(n) => bytes.extend_from_slice(&buffer[..n]),
            }
        }
    });
    Capture { bytes, flooded }
}

/// At most the first 4000 and the last 2000 characters of `text`, for
/// failure messages.
fn shorten(text: &str) -> String {
    const HEAD: usize = 4000;
    const TAIL: usize = 2000;
    let count = text.chars().count();
    if count <= HEAD + TAIL {
        return text.to_string();
    }
    let head: String = text.chars().take(HEAD).collect();
    let tail: String = text.chars().skip(count - TAIL).collect();
    format!(
        "{head}\n[... {} characters left out ...]\n{tail}",
        count - HEAD - TAIL
    )
}

/// Runs `tansy ARGS` in `dir` with `stdin` as its whole standard input and
/// waits for it to end, at most `timeout`.
#[track_caller]
pub fn run_in(dir: &Path, args: &[&str], stdin: &[u8], timeout: Duration, what: &str) -> Run {
    let mut child = Reaper(spawn(dir, args));

    // Feed standard input from a thread: a program that never reads must not
    // block the test. Dropping the handle closes the pipe (end of input).
    let mut child_stdin = child.0.stdin.take().expect("stdin is piped");
    let input = stdin.to_vec();
    let writer = thread::spawn(move || {
        let _ = child_stdin.write_all(&input);
    });
    let stdout = read_all(child.0.stdout.take().expect("stdout is piped"));
    let stderr = read_all(child.0.stderr.take().expect("stderr is piped"));

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.0.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if stdout.flooded() || stderr.flooded() => {
                break Err(format!(
                    "the interpreter wrote more than {} MiB of output and was stopped. \
                     Is there a loop that never ends?",
                    OUTPUT_LIMIT / (1024 * 1024)
                ));
            }
            Ok(None) if Instant::now() >= deadline => {
                break Err(format!(
                    "the interpreter did not finish within {} seconds and was stopped.",
                    timeout.as_secs()
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(2)),
            Err(e) => panic!("cannot wait for the interpreter: {e}"),
        }
    };
    if status.is_err() {
        // Ending the process closes its pipes, which ends the reader threads.
        let _ = child.0.kill();
        let _ = child.0.wait();
    }
    let _ = writer.join();
    let stdout = stdout.finish();
    let stderr = stderr.finish();

    let status = status.unwrap_or_else(|problem| {
        panic!(
            "{problem}\n{what}\n\
             --- standard output so far ---\n{}\n--- standard error so far ---\n{}",
            shorten(&stdout),
            shorten(&stderr)
        )
    });
    Run {
        stdout,
        stderr,
        code: status.code(),
        what: what.to_string(),
    }
}

fn describe_script(source: &str) -> String {
    format!("--- script (main.tn) ---\n{source}\n--- end of script ---")
}

/// Runs `source` as the script `main.tn`, with the current directory set to
/// the script's directory and no input.
#[track_caller]
pub fn run_script(source: &str) -> Run {
    run_script_with_input(source, b"")
}

/// Like [`run_script`], with `stdin` as the standard input.
#[track_caller]
pub fn run_script_with_input(source: &str, stdin: &[u8]) -> Run {
    run_script_for(source, stdin, TIMEOUT)
}

#[track_caller]
pub fn run_script_for(source: &str, stdin: &[u8], timeout: Duration) -> Run {
    let dir = TempDir::new();
    dir.write("main.tn", source);
    run_in(
        dir.path(),
        &["main.tn"],
        stdin,
        timeout,
        &describe_script(source),
    )
}

/// Starts the REPL (`tansy` without arguments), sends `input` as the whole
/// standard input, and waits for the REPL to end.
#[track_caller]
pub fn run_repl(input: &str) -> Run {
    let dir = TempDir::new();
    let what = format!("--- input to the REPL ---\n{input}\n--- end of input ---");
    run_in(dir.path(), &[], input.as_bytes(), TIMEOUT, &what)
}

/// Runs `source` as the script `main.tn`, reads the first `keep` bytes of
/// its standard output and then closes the reading end of that pipe, as
/// `tansy main.tn | head -1` does. Waits for the interpreter to end, at
/// most [`TIMEOUT`].
///
/// In the result, `stdout` holds only what was read before the pipe was
/// closed.
#[track_caller]
pub fn run_and_close_output(source: &str, keep: usize) -> Run {
    let dir = TempDir::new();
    dir.write("main.tn", source);
    let mut child = Reaper(spawn(dir.path(), &["main.tn"]));
    // No input: `input()` would see the end of input at once.
    drop(child.0.stdin.take());
    let mut out = child.0.stdout.take().expect("stdout is piped");
    let stderr = read_all(child.0.stderr.take().expect("stderr is piped"));

    // Read on a thread, so that an interpreter that prints nothing cannot
    // block the test: the read ends at the latest when the child is killed.
    let first = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = out.by_ref().take(keep as u64).read_to_end(&mut bytes);
        // Dropping `out` here closes the pipe. The next write of the
        // interpreter fails.
        bytes
    });

    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        match child.0.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() >= deadline => break None,
            Ok(None) => thread::sleep(Duration::from_millis(2)),
            Err(e) => panic!("cannot wait for the interpreter: {e}"),
        }
    };
    if status.is_none() {
        let _ = child.0.kill();
        let _ = child.0.wait();
    }
    let stdout = String::from_utf8_lossy(&first.join().expect("reader thread")).into_owned();
    let stderr = stderr.finish();
    let what = describe_script(source);
    let Some(status) = status else {
        panic!(
            "the interpreter was still running {} seconds after the test started to read the first \
             {keep} bytes of its output (after those, the test closes the pipe); it was stopped.\n{what}\n\
             --- standard output that was read ---\n{stdout}\n--- standard error ---\n{stderr}",
            TIMEOUT.as_secs()
        );
    };
    Run {
        stdout,
        stderr,
        code: status.code(),
        what,
    }
}

// ---------------------------------------------------------------------------
// Expectations
// ---------------------------------------------------------------------------

/// The diagnostic of SPEC 11.2 for an error in `main.tn`: the header line,
/// the source line and the marker line.
///
/// `source_line` is the line as it must be printed (without the four spaces
/// of indentation). The marker is `width` carets under column `column`.
pub fn diagnostic(
    line: usize,
    column: usize,
    message: &str,
    source_line: &str,
    width: usize,
) -> String {
    diagnostic_in("main.tn", line, column, message, source_line, width)
}

/// Like [`diagnostic`], for another file name (`<repl>` for the REPL).
pub fn diagnostic_in(
    file: &str,
    line: usize,
    column: usize,
    message: &str,
    source_line: &str,
    width: usize,
) -> String {
    format!(
        "{file}:{line}:{column}: error: {message}\n    {source_line}\n    {}{}\n",
        " ".repeat(column - 1),
        "^".repeat(width)
    )
}

fn status_name(code: Option<i32>) -> String {
    match code {
        Some(OK) => "0 (success)".into(),
        Some(RUNTIME_ERROR) => "1 (runtime error)".into(),
        Some(COMPILE_ERROR) => "2 (syntax or name error)".into(),
        Some(USAGE_OR_IO_ERROR) => "3 (usage or input/output error)".into(),
        Some(101) => "101 (this is what a Rust panic exits with: the interpreter crashed)".into(),
        Some(other) => other.to_string(),
        None => "none: the process was ended by a signal, which means the interpreter crashed \
                 (a stack overflow or an abort)"
            .into(),
    }
}

/// The part of a failure message for one stream that is not as expected.
/// Very long texts are shown with their middle left out.
fn differs(stream: &str, short: &str, expected: &str, actual: &str) -> String {
    let (expected, actual) = (shorten(expected), shorten(actual));
    format!(
        "{stream} differs.\n--- expected {short} ---\n{expected}\n--- actual {short} ---\n{actual}\n\
         --- the same, with invisible characters shown ---\nexpected: {expected:?}\n  actual: {actual:?}"
    )
}

impl Run {
    /// Fails the test unless the three results are exactly as given.
    #[track_caller]
    pub fn expect(&self, stdout: &str, stderr: &str, code: i32) {
        let mut problems = Vec::new();
        if self.code != Some(code) {
            problems.push(format!(
                "exit status: expected {}, got {}",
                status_name(Some(code)),
                status_name(self.code)
            ));
        }
        if self.stdout != stdout {
            problems.push(differs("standard output", "stdout", stdout, &self.stdout));
        }
        if self.stderr != stderr {
            problems.push(differs("standard error", "stderr", stderr, &self.stderr));
        }
        if !problems.is_empty() {
            panic!("\n{}\n\n{}\n", self.what, problems.join("\n\n"));
        }
    }

    /// Fails the test unless the run ended with `code`, wrote exactly
    /// `stdout`, and wrote a standard error that begins with `stderr_start`.
    #[track_caller]
    pub fn expect_stderr_start(&self, stdout: &str, stderr_start: &str, code: i32) {
        if self.stderr.starts_with(stderr_start) {
            self.expect(stdout, &self.stderr, code);
        } else {
            // Report it through the usual message.
            self.expect(stdout, stderr_start, code);
        }
    }
}

/// The script must run to the end, print exactly `stdout`, print nothing on
/// standard error and exit with status 0.
#[track_caller]
pub fn expect_output(source: &str, stdout: &str) {
    run_script(source).expect(stdout, "", OK);
}

/// The script must be rejected before it runs: nothing on standard output,
/// exactly `stderr` on standard error, exit status 2.
#[track_caller]
pub fn expect_compile_error(source: &str, stderr: &str) {
    run_script(source).expect("", stderr, COMPILE_ERROR);
}

/// The script must print exactly `stdout`, then stop with the runtime error
/// `stderr` and exit status 1.
#[track_caller]
pub fn expect_runtime_error(source: &str, stdout: &str, stderr: &str) {
    run_script(source).expect(stdout, stderr, RUNTIME_ERROR);
}

// ---------------------------------------------------------------------------
// Talking to a running REPL
// ---------------------------------------------------------------------------

/// An interpreter process that the test talks to line by line: the REPL,
/// or a script that reads its input with `input()`.
pub struct Interactive {
    child: Reaper,
    stdin: Option<std::process::ChildStdin>,
    /// Chunks of the REPL's standard output, as they arrive.
    stdout: Receiver<Vec<u8>>,
    stderr: Option<Capture>,
    /// Output that has arrived and that no `expect` has consumed yet.
    pending: Vec<u8>,
    /// Everything sent and received so far, for failure messages.
    transcript: String,
    _dir: TempDir,
}

impl Interactive {
    /// Starts the REPL (`tansy` without arguments).
    pub fn start() -> Interactive {
        Interactive::start_in(TempDir::new(), &[])
    }

    /// Starts `source` as the script `main.tn`. Its standard input stays
    /// open, so the script can wait for lines that the test sends later.
    pub fn start_script(source: &str) -> Interactive {
        let dir = TempDir::new();
        dir.write("main.tn", source);
        let mut script = Interactive::start_in(dir, &["main.tn"]);
        script
            .transcript
            .push_str(&format!("{}\n", describe_script(source)));
        script
    }

    fn start_in(dir: TempDir, args: &[&str]) -> Interactive {
        let mut child = Reaper(spawn(dir.path(), args));
        let stdin = child.0.stdin.take();
        let mut out = child.0.stdout.take().expect("stdout is piped");
        let stderr = Some(read_all(child.0.stderr.take().expect("stderr is piped")));
        let (sender, stdout) = mpsc::channel();
        thread::spawn(move || {
            let mut buffer = [0_u8; 4096];
            loop {
                match out.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if sender.send(buffer[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        Interactive {
            child,
            stdin,
            stdout,
            stderr,
            pending: Vec::new(),
            transcript: String::new(),
            _dir: dir,
        }
    }

    /// Writes `text` to the interpreter's standard input.
    #[track_caller]
    pub fn send(&mut self, text: &str) {
        self.transcript.push_str(&format!("[sent] {text:?}\n"));
        let stdin = self.stdin.as_mut().expect("the input is still open");
        let sent = stdin
            .write_all(text.as_bytes())
            .and_then(|()| stdin.flush());
        if let Err(e) = sent {
            panic!(
                "cannot write to the interpreter (did it exit?): {e}\n{}",
                self.transcript
            );
        }
    }

    /// Waits until the interpreter has written exactly `text` (after
    /// everything earlier calls consumed). Fails if something else arrives,
    /// if the interpreter closes its output, or if nothing arrives within
    /// [`TIMEOUT`].
    ///
    /// The standard input stays open meanwhile. So this only succeeds if
    /// the REPL answers each entry when it gets it, and if the interpreter
    /// flushes its output before it waits for the next line.
    #[track_caller]
    pub fn expect(&mut self, text: &str) {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let shared = self.pending.len().min(text.len());
            if self.pending[..shared] != text.as_bytes()[..shared] {
                panic!(
                    "the interpreter wrote something else.\nexpected next: {text:?}\n  actual next: {:?}\n{}",
                    String::from_utf8_lossy(&self.pending),
                    self.transcript
                );
            }
            if self.pending.len() >= text.len() {
                self.pending.drain(..text.len());
                self.transcript.push_str(&format!("[received] {text:?}\n"));
                return;
            }
            match self
                .stdout
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            {
                Ok(chunk) => self.pending.extend_from_slice(&chunk),
                Err(RecvTimeoutError::Timeout) => panic!(
                    "the interpreter did not write {text:?} within {} seconds; so far it wrote {:?}.\n\
                     Output must be flushed before the interpreter waits for the next line of \
                     input (SPEC 2.3).\n{}",
                    TIMEOUT.as_secs(),
                    String::from_utf8_lossy(&self.pending),
                    self.transcript
                ),
                Err(RecvTimeoutError::Disconnected) => panic!(
                    "the interpreter closed its output while the test waited for {text:?}; it had written {:?}.\n{}",
                    String::from_utf8_lossy(&self.pending),
                    self.transcript
                ),
            }
        }
    }

    /// Closes the interpreter's standard input and waits for the process to
    /// end.
    /// Returns the output that no `expect` consumed, the whole standard
    /// error, and the exit status.
    #[track_caller]
    pub fn finish(mut self) -> Run {
        drop(self.stdin.take());
        let deadline = Instant::now() + TIMEOUT;
        let status = loop {
            match self.child.0.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() >= deadline => panic!(
                    "the interpreter did not exit within {} seconds after its input was closed.\n{}",
                    TIMEOUT.as_secs(),
                    self.transcript
                ),
                Ok(None) => thread::sleep(Duration::from_millis(2)),
                Err(e) => panic!("cannot wait for the interpreter: {e}"),
            }
        };
        // The process has ended, so both pipes reach their end.
        while let Ok(chunk) = self.stdout.recv_timeout(TIMEOUT) {
            self.pending.extend_from_slice(&chunk);
        }
        let stderr = self.stderr.take().expect("finish runs once").finish();
        Run {
            stdout: String::from_utf8_lossy(&self.pending).into_owned(),
            stderr,
            code: status.code(),
            what: format!(
                "--- conversation with the interpreter ---\n{}",
                self.transcript
            ),
        }
    }
}
