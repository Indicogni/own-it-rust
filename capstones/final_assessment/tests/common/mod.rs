//! Helpers shared by the acceptance tests.
//!
//! Nothing here knows how your engine or your program is built. The tests
//! call the library facade of SPEC section 2 and run the `rgx` binary as a
//! separate process, and look only at what a user could see.
//!
//! Four things in this file are worth knowing when a test fails:
//!
//! - **Every test has a time limit.** Library calls run on a helper thread
//!   ([`bounded`]); a run of the binary is stopped after [`RUN_LIMIT`]. A
//!   loop that never ends therefore fails one test with a message. It does
//!   not hang the test run.
//! - **No process is left behind.** A child process is killed when its test
//!   fails or panics. A program that writes without end is stopped after
//!   [`OUTPUT_LIMIT`] bytes, so it cannot fill the memory either.
//! - **Files live in a fresh temporary directory** per test ([`TempDir`]),
//!   which is removed afterwards, also when the test fails.
//! - **Two test files install their own allocator** ([`Watchful`]). It
//!   counts allocations per thread for the allocation budgets of SPEC 8.2,
//!   and it refuses to let the heap grow beyond [`HEAP_CAP`], so that a
//!   pattern which makes your compiler build something enormous ends the
//!   test program with "memory allocation of N bytes failed" instead of
//!   freezing your machine.
//!
//! Each file in `tests/` is its own crate and uses only some of the helpers,
//! so unused-code warnings are switched off here.
#![allow(dead_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rgx::Regex;

// ---------------------------------------------------------------------------
// Time limits
// ---------------------------------------------------------------------------

/// Miri interprets the program and is far slower than a real run. The
/// library tests can run under it (RUBRIC.md, "Unsafe hygiene"); they then
/// use smaller inputs and longer limits.
pub const MIRI: bool = cfg!(miri);

/// How long the library calls of one test may take together. The reference
/// solution needs less than two seconds for every test in a debug build.
pub const LIBRARY_LIMIT: Duration = Duration::from_secs(if MIRI { 900 } else { 30 });

/// How long one run of the binary may take.
pub const RUN_LIMIT: Duration = Duration::from_secs(30);

/// Runs `test` on a helper thread and waits for it, at most
/// [`LIBRARY_LIMIT`].
///
/// If the closure panics, the test fails with that panic. If it is still
/// running at the limit, the test fails with a message that says so; the
/// helper thread is left behind and ends with the test program.
#[track_caller]
pub fn bounded<R: Send + 'static>(test: impl FnOnce() -> R + Send + 'static) -> R {
    let (sender, receiver) = mpsc::channel();
    let helper = thread::spawn(move || {
        let _ = sender.send(test());
    });
    match receiver.recv_timeout(LIBRARY_LIMIT) {
        Ok(value) => value,
        Err(RecvTimeoutError::Timeout) => panic!(
            "TIMEOUT: this test was still running after {} seconds. Matching must take time \
             proportional to the haystack (SPEC 4.5), compiling must respect the limits \
             (SPEC 6), and no call may loop for ever.",
            LIBRARY_LIMIT.as_secs()
        ),
        // The closure panicked: fail with its panic, not with a second one.
        Err(RecvTimeoutError::Disconnected) => match helper.join() {
            Err(panic) => std::panic::resume_unwind(panic),
            Ok(()) => unreachable!("the helper thread ended without sending and without panicking"),
        },
    }
}

// ---------------------------------------------------------------------------
// Calling the library
// ---------------------------------------------------------------------------

/// Compiles a pattern that SPEC section 3 says is valid.
#[track_caller]
pub fn regex(pattern: &str) -> Regex {
    match Regex::new(pattern) {
        Ok(regex) => regex,
        Err(error) => panic!(
            "the pattern {pattern:?} is valid, but Regex::new rejected it: \"{error}\" at offset {}",
            error.offset()
        ),
    }
}

/// The message and offset of the error for a pattern that SPEC section 5
/// says is invalid.
#[track_caller]
pub fn rejection(pattern: &str) -> (String, usize) {
    match Regex::new(pattern) {
        Ok(_) => panic!("the pattern {pattern:?} is invalid, but Regex::new accepted it"),
        Err(error) => (error.to_string(), error.offset()),
    }
}

/// Checks `find` and `is_match` for one pattern and one haystack.
#[track_caller]
pub fn expect_find(pattern: &str, haystack: &str, expected: Option<Range<usize>>) {
    let regex = regex(pattern);
    let found = regex.find(haystack);
    assert_eq!(
        found,
        expected,
        "find: pattern {pattern:?}, haystack {haystack:?}{}",
        show_matches(haystack, &found, &expected)
    );
    assert_eq!(
        regex.is_match(haystack),
        expected.is_some(),
        "is_match: pattern {pattern:?}, haystack {haystack:?}. `is_match` must agree with `find` (SPEC 2.1)"
    );
}

/// The matched texts, to make a failure message easier to read.
fn show_matches(
    haystack: &str,
    found: &Option<Range<usize>>,
    expected: &Option<Range<usize>>,
) -> String {
    let text = |range: &Option<Range<usize>>| match range {
        None => "no match".to_string(),
        Some(range) => match haystack.get(range.clone()) {
            Some(text) => format!("{text:?}"),
            None => "a range that is not on character boundaries of the haystack".to_string(),
        },
    };
    format!(
        "\n  your match:     {}\n  expected match: {}",
        text(found),
        text(expected)
    )
}

/// Checks `find_iter` for one pattern and one haystack.
#[track_caller]
pub fn expect_all(pattern: &str, haystack: &str, expected: &[Range<usize>]) {
    // `take` keeps a `find_iter` that never ends from filling the memory.
    let found: Vec<Range<usize>> = regex(pattern)
        .find_iter(haystack)
        .take(haystack.len() + 2)
        .collect();
    assert_eq!(
        found, expected,
        "find_iter: pattern {pattern:?}, haystack {haystack:?} (SPEC 4.4)"
    );
}

// ---------------------------------------------------------------------------
// Generated input
// ---------------------------------------------------------------------------

/// A small pseudo-random generator with a fixed seed, for the tests that
/// feed generated patterns and haystacks to the library. The same numbers
/// come out on every run and on every machine, so a failure can be repeated.
pub struct Generator(u64);

impl Generator {
    pub fn new(seed: u64) -> Generator {
        // The state must not be zero.
        Generator(seed | 1)
    }

    /// A number from 0 to `limit - 1`.
    pub fn below(&mut self, limit: usize) -> usize {
        // Xorshift, by George Marsaglia: three shifts and three exclusive ors.
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % limit as u64) as usize
    }

    /// One of `items`.
    pub fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }

    /// The concatenation of `count` items, each picked from `items`.
    pub fn text(&mut self, items: &[&str], count: usize) -> String {
        (0..count).map(|_| self.pick(items)).collect()
    }
}

// ---------------------------------------------------------------------------
// The allocator of the two test files that need one
// ---------------------------------------------------------------------------

/// The most heap memory, in bytes, that a test program with the
/// [`Watchful`] allocator may hold at one time.
pub const HEAP_CAP: usize = 1 << 30;

/// Forwards to the system allocator. It counts the allocations of each
/// thread, and it fails any request that would take the live heap of the
/// whole program above [`HEAP_CAP`].
///
/// A test file installs it with
///
/// ```ignore
/// #[global_allocator]
/// static ALLOCATOR: common::Watchful = common::Watchful;
/// ```
pub struct Watchful;

static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn record_allocation() {
    // `try_with`: the allocator also runs while a thread is being torn
    // down, when its thread-local values are gone.
    let _ = COUNTING.try_with(|counting| {
        if counting.get() {
            let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        }
    });
}

/// Reserves `size` more bytes. Returns `false`, and reserves nothing, if
/// that would exceed the cap.
fn reserve(size: usize) -> bool {
    let before = LIVE_BYTES.fetch_add(size, Ordering::Relaxed);
    if before.saturating_add(size) > HEAP_CAP {
        LIVE_BYTES.fetch_sub(size, Ordering::Relaxed);
        return false;
    }
    true
}

// SAFETY: every method forwards its arguments unchanged to `System`, which
// upholds the `GlobalAlloc` contract, or returns null without allocating,
// which the contract allows as the report of a failed allocation. The
// bookkeeping touches an atomic and thread-locals that are initialised with
// constants and have no destructor, so it never allocates and never
// re-enters the allocator.
unsafe impl GlobalAlloc for Watchful {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        if !reserve(layout.size()) {
            return std::ptr::null_mut();
        }
        // SAFETY: the caller upholds the contract of `alloc`, which is
        // passed on unchanged.
        let pointer = unsafe { System.alloc(layout) };
        if pointer.is_null() {
            LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        if !reserve(layout.size()) {
            return std::ptr::null_mut();
        }
        // SAFETY: the caller upholds the contract of `alloc_zeroed`, which
        // is passed on unchanged.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if pointer.is_null() {
            LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record_allocation();
        // A block that grows needs its new size before the old one is
        // given back.
        if !reserve(new_size) {
            return std::ptr::null_mut();
        }
        // SAFETY: the caller upholds the contract of `realloc`, which is
        // passed on unchanged.
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        // On success the old block is gone; on failure the new one never
        // existed.
        LIVE_BYTES.fetch_sub(
            if moved.is_null() {
                new_size
            } else {
                layout.size()
            },
            Ordering::Relaxed,
        );
        moved
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        // SAFETY: the caller upholds the contract of `dealloc`, which is
        // passed on unchanged.
        unsafe { System.dealloc(pointer, layout) }
    }
}

/// Runs `work` and returns its result together with the number of
/// allocations and reallocations it made **on this thread**. The count is
/// only meaningful in a test file that installs [`Watchful`]; call
/// [`assert_counting_works`] once to be sure.
pub fn count_allocations<R>(work: impl FnOnce() -> R) -> (R, usize) {
    let before = ALLOCATIONS.with(Cell::get);
    let was_counting = COUNTING.with(|counting| counting.replace(true));
    let result = work();
    COUNTING.with(|counting| counting.set(was_counting));
    (result, ALLOCATIONS.with(Cell::get) - before)
}

/// Fails if allocations are not being counted, so that a budget test cannot
/// pass only because nothing counts.
#[track_caller]
pub fn assert_counting_works() {
    let (boxed, allocations) = count_allocations(|| std::hint::black_box(Box::new(7_u64)));
    assert_eq!(*boxed, 7);
    assert_eq!(
        allocations, 1,
        "this test file must install `common::Watchful` as its global allocator"
    );
}

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
                "rgx-test-{}-{}-{nanos:09}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            );
            let path = base.join(name);
            match fs::create_dir(&path) {
                Ok(()) => return TempDir(path),
                // Somebody else has this name: take the next one.
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!(
                    "cannot create a temporary directory in {}: {error}",
                    base.display()
                ),
            }
        }
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Writes `contents` to the file `name` inside the directory. `name`
    /// uses `/` between directories, on every platform; missing directories
    /// are created.
    pub fn write(&self, name: &str, contents: impl AsRef<[u8]>) {
        let path = self.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("cannot create {}: {e}", parent.display()));
        }
        fs::write(&path, contents)
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    }

    /// Creates the (empty) directory `name`, and its parents.
    pub fn create_dir(&self, name: &str) {
        let path = self.join(name);
        fs::create_dir_all(&path)
            .unwrap_or_else(|e| panic!("cannot create {}: {e}", path.display()));
    }

    fn join(&self, name: &str) -> PathBuf {
        name.split('/')
            .fold(self.0.clone(), |path, part| path.join(part))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// ---------------------------------------------------------------------------
// Running the binary
// ---------------------------------------------------------------------------

/// The path of the binary under test.
pub const RGX: &str = env!("CARGO_BIN_EXE_rgx");

/// The exit statuses of SPEC 7.6.
pub const SELECTED: i32 = 0;
pub const NOTHING_SELECTED: i32 = 1;
pub const FAILED: i32 = 2;

/// Kills and reaps the child process when dropped, so that a failing test
/// never leaves an `rgx` running.
pub struct Reaper(pub Child);

impl Drop for Reaper {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// What one finished run of the binary produced.
pub struct Run {
    pub stdout: String,
    pub stderr: String,
    /// The exit status, or `None` if the process was ended by a signal
    /// (on Unix that is what a crash looks like).
    pub code: Option<i32>,
    /// What was run, for failure messages.
    what: String,
}

/// Starts `rgx ARGS` with `dir` as its current directory and all three
/// standard streams as pipes. An argument is a `&str` in almost every test;
/// `&OsStr` allows one that is not valid Unicode.
pub fn spawn<A: AsRef<OsStr>>(dir: &Path, args: &[A]) -> Reaper {
    let child = Command::new(RGX)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot start {RGX}: {e}"));
    Reaper(child)
}

/// The most bytes that a test keeps of one output stream of the program.
/// The largest output that any test expects is about a third of this.
pub const OUTPUT_LIMIT: usize = 32 << 20;

/// Collects what the program writes to `stream`, up to a little more than
/// [`OUTPUT_LIMIT`] bytes. Then it stops reading and closes its end of the
/// pipe: a program that prints in an endless loop must not make the test
/// program hold gigabytes.
fn read_all(stream: impl Read + Send + 'static) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stream.take(OUTPUT_LIMIT as u64 + 1).read_to_end(&mut bytes);
        bytes
    })
}

/// Waits until the child has ended, at most `limit`. Returns `None`, after
/// killing the child, if it has not.
pub fn wait_with_limit(child: &mut Child, limit: Duration) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + limit;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if Instant::now() >= deadline => {
                // Ending the process closes its pipes, which ends the
                // threads that read from them.
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => thread::sleep(Duration::from_millis(2)),
            Err(error) => panic!("cannot wait for rgx: {error}"),
        }
    }
}

/// Runs `rgx ARGS` in `dir` with `stdin` as its whole standard input and
/// waits for it to end.
#[track_caller]
pub fn run_in<A: AsRef<OsStr>>(dir: &Path, args: &[A], stdin: &[u8]) -> Run {
    let what = format!(
        "--- command ---\nrgx {}\n--- end of command ---",
        shell_words(args)
    );
    let mut child = spawn(dir, args);

    // Feed standard input from a thread: a program that never reads must
    // not block the test. Dropping the handle closes the pipe.
    let mut child_stdin = child.0.stdin.take().expect("stdin is piped");
    let input = stdin.to_vec();
    let writer = thread::spawn(move || {
        let _ = child_stdin.write_all(&input);
    });
    let stdout = read_all(child.0.stdout.take().expect("stdout is piped"));
    let stderr = read_all(child.0.stderr.take().expect("stderr is piped"));

    let status = wait_with_limit(&mut child.0, RUN_LIMIT);
    let _ = writer.join();
    let mut stdout = stdout.join().expect("reader thread");
    let mut stderr = stderr.join().expect("reader thread");
    let flooded = stdout.len().max(stderr.len()) > OUTPUT_LIMIT;
    if flooded {
        // Only the start is shown; do not copy megabytes to show it.
        stdout.truncate(8 << 10);
        stderr.truncate(8 << 10);
    }
    let stdout = String::from_utf8_lossy(&stdout).into_owned();
    let stderr = String::from_utf8_lossy(&stderr).into_owned();

    if flooded {
        panic!(
            "rgx wrote more than {} MiB, far more than any test expects, and the test stopped \
             reading. Look for a loop that prints without end.\n{what}\n\
             --- start of standard output ---\n{}\n--- start of standard error ---\n{}",
            OUTPUT_LIMIT >> 20,
            shorten(&stdout),
            shorten(&stderr)
        );
    }
    let Some(status) = status else {
        panic!(
            "TIMEOUT: rgx did not finish within {} seconds and was stopped.\n{what}\n\
             --- standard output so far ---\n{}\n--- standard error so far ---\n{}",
            RUN_LIMIT.as_secs(),
            shorten(&stdout),
            shorten(&stderr)
        );
    };
    Run {
        stdout,
        stderr,
        code: status.code(),
        what,
    }
}

/// Runs `rgx ARGS` in an empty temporary directory with `stdin` as its
/// standard input.
#[track_caller]
pub fn run(args: &[&str], stdin: &str) -> Run {
    let dir = TempDir::new();
    run_in(dir.path(), args, stdin.as_bytes())
}

fn shell_words<A: AsRef<OsStr>>(args: &[A]) -> String {
    args.iter()
        .map(|arg| format!("{:?}", arg.as_ref()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Long output is cut in failure messages; nobody reads megabytes.
fn shorten(text: &str) -> String {
    const KEEP: usize = 4000;
    if text.len() <= KEEP {
        return text.to_string();
    }
    let mut end = KEEP;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}\n[... {} more bytes not shown ...]",
        &text[..end],
        text.len() - end
    )
}

fn status_name(code: Option<i32>) -> String {
    match code {
        Some(SELECTED) => "0 (a line was selected)".into(),
        Some(NOTHING_SELECTED) => "1 (no line was selected)".into(),
        Some(FAILED) => "2 (an error)".into(),
        Some(100) => "100 (the starter's \"nothing is implemented yet\")".into(),
        Some(101) => "101 (this is what a Rust panic exits with: rgx crashed)".into(),
        Some(other) => other.to_string(),
        None => "none: the process was ended by a signal, which means rgx crashed \
                 (a stack overflow or an abort)"
            .into(),
    }
}

impl Run {
    fn problems(&self, stdout: &str, code: i32) -> Vec<String> {
        let mut problems = Vec::new();
        if self.code != Some(code) {
            problems.push(format!(
                "exit status: expected {}, got {}",
                status_name(Some(code)),
                status_name(self.code)
            ));
        }
        if self.stdout != stdout {
            problems.push(difference("standard output", stdout, &self.stdout));
        }
        problems
    }

    #[track_caller]
    fn fail_if(&self, problems: Vec<String>) {
        if !problems.is_empty() {
            panic!(
                "\n{}\n\n{}\n\n--- standard error of this run ---\n{}",
                self.what,
                problems.join("\n\n"),
                shorten(&self.stderr)
            );
        }
    }

    /// The run must have printed exactly `stdout`, nothing on standard
    /// error, and ended with `code`.
    #[track_caller]
    pub fn expect(&self, stdout: &str, code: i32) {
        let mut problems = self.problems(stdout, code);
        if !self.stderr.is_empty() {
            problems.push("standard error must be empty".to_string());
        }
        self.fail_if(problems);
    }

    /// The run must have printed exactly `stdout` and exactly `stderr`,
    /// and ended with `code`.
    #[track_caller]
    pub fn expect_with_stderr(&self, stdout: &str, stderr: &str, code: i32) {
        let mut problems = self.problems(stdout, code);
        if self.stderr != stderr {
            problems.push(difference("standard error", stderr, &self.stderr));
        }
        self.fail_if(problems);
    }

    /// The run must have printed exactly `stdout` and ended with status 2.
    /// Standard error must consist of one line per entry of `prefixes`, in
    /// that order, each line starting with its prefix.
    #[track_caller]
    pub fn expect_error_lines(&self, stdout: &str, prefixes: &[&str]) {
        let mut problems = self.problems(stdout, FAILED);
        let lines: Vec<&str> = self.stderr.lines().collect();
        let fits = lines.len() == prefixes.len()
            && lines
                .iter()
                .zip(prefixes)
                .all(|(line, prefix)| line.starts_with(prefix))
            && self.stderr.ends_with('\n');
        if !fits {
            problems.push(format!(
                "standard error must have {} line(s), starting with\n{}",
                prefixes.len(),
                prefixes
                    .iter()
                    .map(|p| format!("    {p}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        self.fail_if(problems);
    }
}

fn difference(stream: &str, expected: &str, actual: &str) -> String {
    let mut text = format!(
        "{stream} differs.\n--- expected ---\n{}\n--- actual ---\n{}",
        shorten(expected),
        shorten(actual)
    );
    // The first line that differs says more than two long listings.
    let mut expected_lines = expected.split_inclusive('\n');
    let mut actual_lines = actual.split_inclusive('\n');
    let mut number = 1;
    loop {
        match (expected_lines.next(), actual_lines.next()) {
            (Some(e), Some(a)) if e == a => number += 1,
            (None, None) => break,
            (e, a) => {
                text.push_str(&format!(
                    "\n--- first difference: line {number} ---\nexpected: {}\n  actual: {}",
                    e.map_or("(no more output)".to_string(), |line| format!("{line:?}")),
                    a.map_or("(no more output)".to_string(), |line| format!("{line:?}"))
                ));
                break;
            }
        }
    }
    text
}
