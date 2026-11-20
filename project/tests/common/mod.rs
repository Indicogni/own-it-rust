//! Helpers shared by the milestone tests. Every `tests/mNN_*.rs` file is its
//! own test crate and includes this module with `mod common;`. You do not
//! need to edit it.
#![allow(dead_code)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

// The size predictions (`size_of::<&str>()` and so on) and the counting
// allocator below are written for the 64-bit targets the course supports
// (Linux, macOS and Windows on x86-64 or AArch64). Fail loudly anywhere else
// instead of producing confusing prediction failures.
const _: () = assert!(
    size_of::<usize>() == 8,
    "the loglens tests assume a 64-bit target"
);

/// Placeholder for a prediction. Replace the call with your answer before
/// running the milestone, for example `let p1: usize = 5;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

/// Runs `f` on a helper thread and returns its result, or fails the test
/// with a clear message if it has not finished after `secs` seconds.
///
/// The milestone tests use this around code that would otherwise block
/// forever when a solution keeps a channel sender alive, never wakes a task,
/// or deadlocks. `what` names the thing being run; it appears in the failure
/// message and as the helper thread's name. If `f` panics (a failed
/// assertion, a `todo!()`, an unanswered `guess()`), the same panic continues
/// in the test, so the original message is the one you read.
///
/// `f` runs on another thread, so "the calling thread" inside `f` is the
/// helper thread, and allocation counts must be taken inside `f`.
#[track_caller]
pub fn within<R: Send + 'static>(
    secs: u64,
    what: &str,
    f: impl FnOnce() -> R + Send + 'static,
) -> R {
    let (tx, rx) = mpsc::channel();
    let helper = std::thread::Builder::new()
        .name(what.to_string())
        .spawn(move || {
            let _ = tx.send(f());
        })
        .expect("spawn the helper thread");
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(value) => value,
        Err(RecvTimeoutError::Timeout) => panic!(
            "{what} was still running after {secs} s. It is probably blocked forever: \
             look for a channel sender that is never dropped, a task that is never \
             woken, a lock that is never released, or a loop that never ends."
        ),
        // The sender was dropped without a value: `f` panicked. Re-raise its
        // panic here so `#[should_panic]` and the test report see the cause.
        Err(RecvTimeoutError::Disconnected) => match helper.join() {
            Err(panic) => std::panic::resume_unwind(panic),
            Ok(()) => unreachable!("the helper sends before it returns"),
        },
    }
}

/// A child process that is killed and reaped when the guard is dropped.
///
/// `std::process::Child` keeps running when it is dropped. A test that
/// panics while its child is alive would leave that process behind, so the
/// tests that start a long-running `loglens` hold it in this guard.
pub struct KillOnDrop(pub Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        // Both calls fail harmlessly if the child has already exited.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Runs `command` to its end with `stdin` as its standard input and returns
/// its exit code, stdout, and stderr.
///
/// If the program is still running after `secs` seconds, it is killed and
/// the test fails with a message, instead of the test suite hanging on a
/// program that never exits. The test also fails, and shows what the program
/// wrote to stderr, if the program panicked (a Rust program that panics
/// exits with status 101, which `loglens` never uses otherwise) or did not
/// exit by itself (on Unix: if a signal ended it).
#[track_caller]
pub fn output_within(secs: u64, mut command: Command, stdin: &[u8]) -> (i32, Vec<u8>, Vec<u8>) {
    let name = Path::new(command.get_program())
        .file_name()
        .unwrap_or(command.get_program())
        .to_string_lossy()
        .into_owned();
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = KillOnDrop(command.spawn().expect("start the program"));

    // Feed stdin and drain both outputs on helper threads, so that a full
    // pipe can never block the program or this test.
    let mut input = child.0.stdin.take().expect("stdin is piped");
    let bytes = stdin.to_vec();
    std::thread::spawn(move || {
        // The program may exit without reading its input; that is not an error.
        let _ = input.write_all(&bytes);
    });
    fn drain(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            bytes
        })
    }
    let stdout = drain(child.0.stdout.take().expect("stdout is piped"));
    let stderr = drain(child.0.stderr.take().expect("stderr is piped"));

    let deadline = Instant::now() + Duration::from_secs(secs);
    let status = loop {
        if let Some(status) = child.0.try_wait().expect("wait for the program") {
            break status;
        }
        if Instant::now() >= deadline {
            // Unwinding drops `child`, which kills the program.
            panic!(
                "{name} was still running after {secs} s and was killed. It is probably \
                 blocked forever: look for a read from an input that never ends, a channel \
                 sender that is never dropped, or a loop that never ends."
            );
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let stdout = stdout.join().expect("read stdout");
    let stderr = stderr.join().expect("read stderr");
    let Some(code) = status.code() else {
        panic!("{name} did not exit by itself: {status}");
    };
    if code == PANIC_STATUS {
        panic!(
            "{name} panicked (exit status {PANIC_STATUS}) instead of finishing. \
             It wrote this to stderr:\n{}",
            String::from_utf8_lossy(&stderr)
        );
    }
    (code, stdout, stderr)
}

/// The exit status of a Rust program whose main thread panicked, for
/// example at a `todo!()`.
const PANIC_STATUS: i32 = 101;

/// Whether `inner` lies entirely inside `outer`'s bytes: the zero-copy check.
pub fn borrows_from(outer: &str, inner: &str) -> bool {
    let outer = outer.as_bytes().as_ptr_range();
    let inner = inner.as_bytes().as_ptr_range();
    outer.start <= inner.start && inner.end <= outer.end
}

/// The path of a file in `project/fixtures/`.
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

/// The contents of a UTF-8 file in `project/fixtures/`.
pub fn fixture_text(name: &str) -> String {
    std::fs::read_to_string(fixture(name)).expect("fixture exists")
}

/// A fresh directory under the system temporary directory, removed on drop.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> TempDir {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("loglens-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temporary directory");
        TempDir(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Write `contents` to a file in this directory and return its path.
    pub fn file(&self, name: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, contents).expect("write temporary file");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub mod alloc {
    //! A global allocator that counts allocations per thread, so tests can
    //! check *how* a task was solved. Counts cover only the calling thread,
    //! so tests running in parallel do not disturb each other. The optimizer
    //! may remove allocations in release builds, so tests assert upper bounds.
    //!
    //! [`count`] gives the number of allocations; [`count_bytes`] gives the
    //! number of bytes requested, which tells a small bounded buffer from a
    //! copy of everything.

    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static ACTIVE: Cell<bool> = const { Cell::new(false) };
        static COUNT: Cell<usize> = const { Cell::new(0) };
        static BYTES: Cell<usize> = const { Cell::new(0) };
    }

    /// Forwards to the system allocator and counts allocations and growth.
    pub struct Counting;

    fn record(bytes: usize) {
        let _ = ACTIVE.try_with(|active| {
            if active.get() {
                let _ = COUNT.try_with(|count| count.set(count.get() + 1));
                let _ = BYTES.try_with(|total| total.set(total.get() + bytes));
            }
        });
    }

    // SAFETY: every method forwards unchanged to `System`, which upholds the
    // GlobalAlloc contract. Recording touches only const-initialized
    // thread-locals without destructors, so it never allocates or reenters.
    // That relies on the target having native thread-local storage, which
    // every supported target has (64-bit Linux, macOS and Windows). On a
    // target that emulates thread-locals with heap-allocated keys, the first
    // access could allocate inside `alloc`; this allocator is not meant for
    // such targets.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record(layout.size());
            unsafe { System.alloc(layout) }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record(layout.size());
            unsafe { System.alloc_zeroed(layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            record(new_size);
            unsafe { System.realloc(ptr, layout, new_size) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    /// Runs `f` and returns its result with the number of allocations and
    /// reallocations it made on this thread.
    pub fn count<R>(f: impl FnOnce() -> R) -> (R, usize) {
        let before = COUNT.with(Cell::get);
        let was_active = ACTIVE.with(|a| a.replace(true));
        let result = f();
        ACTIVE.with(|a| a.set(was_active));
        (result, COUNT.with(Cell::get) - before)
    }

    /// Runs `f` and returns its result with the number of bytes it asked the
    /// allocator for on this thread (a regrown buffer counts at its new
    /// size each time).
    pub fn count_bytes<R>(f: impl FnOnce() -> R) -> (R, usize) {
        let before = BYTES.with(Cell::get);
        let was_active = ACTIVE.with(|a| a.replace(true));
        let result = f();
        ACTIVE.with(|a| a.set(was_active));
        (result, BYTES.with(Cell::get) - before)
    }
}

#[global_allocator]
static ALLOC: alloc::Counting = alloc::Counting;

#[allow(unused_imports)]
pub use alloc::{count, count_bytes};
