// Helpers shared by the tests of this package. You do not need to edit this file.
//
// The unit tests in `src/` reach it as `crate::test_support`. The integration tests
// in `tests/` are separate crates, so each of them pulls the same file in with
// `mod test_support { include!("../src/test_support.rs"); }`. Each test crate uses
// only some of these helpers, so the module is declared with `#[allow(dead_code)]`.

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;
use tokio::runtime::{Builder, Runtime};

/// How long a single test may run before it is reported as stuck.
const LIMIT: Duration = Duration::from_secs(60);

/// Runs `f` on a helper thread and returns its result. If `f` has not finished after
/// 60 seconds, the test fails with a clear message instead of hanging the whole run.
///
/// The tests use this around code that waits forever when a solution has a mistake:
/// a queue whose sender is never dropped, a loop that never ends, a lock that is
/// taken twice, a server that never stops. If `f` panics (a failed assertion, a
/// `todo!()`, an unanswered `guess()`), the same panic continues in the test, so the
/// message you read is the original one.
#[track_caller]
pub fn within<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    // The test harness names each test's thread after the test.
    let test = std::thread::current();
    let what = test.name().unwrap_or("this test").to_owned();
    let (done, finished) = mpsc::channel();
    let helper = std::thread::Builder::new()
        .name(what.clone())
        .spawn(move || {
            let _ = done.send(f());
        })
        .expect("spawn the helper thread");
    match finished.recv_timeout(LIMIT) {
        Ok(value) => value,
        Err(RecvTimeoutError::Timeout) => panic!(
            "{what} was still running after {} s. It is probably blocked forever: look \
             for a channel sender that is never dropped, a loop that never ends, a lock \
             that is taken twice, or a server that never stops.",
            LIMIT.as_secs()
        ),
        // The helper dropped the sender without sending: `f` panicked. Continue
        // that panic here, so the test report shows the real message.
        Err(RecvTimeoutError::Disconnected) => match helper.join() {
            Err(panic) => std::panic::resume_unwind(panic),
            Ok(()) => unreachable!("the helper sends before it returns"),
        },
    }
}

/// Runs an async test body to completion on a runtime built by `runtime`, under the
/// same 60-second limit as [`within`]. Use it as
/// `run(paused_time, || async { ... })`.
#[track_caller]
pub fn run<F: Future<Output = ()>>(
    runtime: fn() -> Runtime,
    body: impl FnOnce() -> F + Send + 'static,
) {
    within(move || runtime().block_on(body()));
}

/// A current-thread runtime with the real clock.
pub fn real_time() -> Runtime {
    Builder::new_current_thread().enable_all().build().unwrap()
}

/// A current-thread runtime with virtual time, like
/// `#[tokio::test(start_paused = true)]`: the clock jumps to the next timer whenever
/// every task is waiting, so sleeps cost nothing and elapsed times are exact.
/// Only for tests without sockets: waiting for the network also looks like "every
/// task is waiting", and the clock would jump.
pub fn paused_time() -> Runtime {
    Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()
        .unwrap()
}

/// A multi-thread runtime with four worker threads, as a server would use.
pub fn worker_threads() -> Runtime {
    Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
}

/// A private directory for one test's files. It is removed when the value is
/// dropped, so the tests leave nothing behind in the system temp directory.
pub struct TempDir(pub PathBuf);

impl TempDir {
    /// Creates a fresh directory. `test` becomes part of its name.
    pub fn new(test: &str) -> TempDir {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.subsec_nanos());
        let name = format!("ecosystem-{test}-{}-{nonce}", std::process::id());
        let dir = std::env::temp_dir().join(name);
        // `create_dir`, not `create_dir_all`: never reuse a directory that
        // somebody else has already created under this name.
        std::fs::create_dir(&dir).expect("create a fresh temp directory");
        TempDir(dir)
    }

    /// Writes `contents` to a file called `name` in the directory and returns its path.
    pub fn file(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
