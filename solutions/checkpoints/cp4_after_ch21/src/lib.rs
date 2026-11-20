//! Checkpoint 4: `seal`, a file integrity checker. Reference solution.
//!
//! `COMMENTARY.md` next to this crate explains the decisions. Short version:
//! bytes and text are streamed through fixed buffers and never collected;
//! the workers of `check_all` share one atomic counter and borrow everything
//! else through `thread::scope`; and both ends of the async part keep their
//! state behind one lock or one flag, so that "is it done?" and "who must be
//! woken?" are always answered together.

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::future::Future;
use std::io::{self, BufRead, Read, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::pin::{Pin, pin};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};

// ----------------------------------------------------------------- digest

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// The 64-bit FNV-1a hash of everything `reader` gives. SPEC section 2.
pub fn digest<R: Read>(mut reader: R) -> io::Result<u64> {
    let mut hash = FNV_OFFSET;
    // One fixed buffer on the stack: the memory in use does not depend on
    // the length of the input, and each `read` asks for 8 KiB.
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            // Only zero bytes mean the end. A short read is normal.
            Ok(0) => return Ok(hash),
            Ok(count) => {
                for &byte in &buffer[..count] {
                    hash = (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME);
                }
            }
            // Nothing was read and nothing is lost: ask again.
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

// ---------------------------------------------------------------- entries

/// One line of a manifest: the digest of a file and its name. SPEC section 3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub digest: u64,
    pub name: String,
}

/// Number of hexadecimal digits of a digest.
const DIGITS: usize = 16;
const SEPARATOR: &str = "  ";

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}{SEPARATOR}", self.digest)?;
        // The name goes out in pieces: the text up to the next character
        // that needs an escape, then the escape. No escaped copy is built.
        let mut rest = self.name.as_str();
        while let Some(at) = rest.find(['\\', '\n', '\r']) {
            let (plain, tail) = rest.split_at(at);
            f.write_str(plain)?;
            f.write_str(match tail.as_bytes()[0] {
                b'\\' => "\\\\",
                b'\n' => "\\n",
                _ => "\\r",
            })?;
            // The three characters are one byte long in UTF-8.
            rest = &tail[1..];
        }
        f.write_str(rest)
    }
}

/// Why a line is not an entry. SPEC section 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryError {
    Digest,
    Separator,
    Name,
    /// The byte offset, in the line, of the backslash that starts an
    /// unknown escape.
    Escape(usize),
}

impl fmt::Display for EntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntryError::Digest => f.write_str("expected 16 hexadecimal digits"),
            EntryError::Separator => f.write_str("expected two spaces after the digest"),
            EntryError::Name => f.write_str("the name is empty"),
            EntryError::Escape(offset) => write!(f, "unknown escape at byte {offset}"),
        }
    }
}

impl Error for EntryError {}

impl FromStr for Entry {
    type Err = EntryError;

    fn from_str(line: &str) -> Result<Entry, EntryError> {
        // `split_at_checked` is `None` when the line is too short, and also
        // when byte 16 is inside a character. `&line[..16]` would panic.
        let (digits, rest) = line.split_at_checked(DIGITS).ok_or(EntryError::Digest)?;
        // `from_str_radix` alone would accept a leading `+`.
        if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(EntryError::Digest);
        }
        let digest = u64::from_str_radix(digits, 16).map_err(|_| EntryError::Digest)?;
        let escaped = rest.strip_prefix(SEPARATOR).ok_or(EntryError::Separator)?;
        if escaped.is_empty() {
            return Err(EntryError::Name);
        }

        // The name is never longer than its escaped form: one allocation.
        let mut name = String::with_capacity(escaped.len());
        let mut chars = escaped.char_indices();
        while let Some((at, c)) = chars.next() {
            if c != '\\' {
                name.push(c);
                continue;
            }
            name.push(match chars.next() {
                Some((_, '\\')) => '\\',
                Some((_, 'n')) => '\n',
                Some((_, 'r')) => '\r',
                // `at` counts from the start of the name; the error counts
                // from the start of the line.
                _ => return Err(EntryError::Escape(DIGITS + SEPARATOR.len() + at)),
            });
        }
        Ok(Entry { digest, name })
    }
}

// -------------------------------------------------------------- manifests

/// Why a manifest could not be read. SPEC section 4.
#[derive(Debug)]
pub enum ManifestError {
    Io(io::Error),
    Line { number: usize, error: EntryError },
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // The details are in `source()`, not repeated here.
            ManifestError::Io(_) => f.write_str("cannot read the manifest"),
            ManifestError::Line { number, .. } => {
                write!(f, "line {number} of the manifest is not valid")
            }
        }
    }
}

impl Error for ManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ManifestError::Io(error) => Some(error),
            ManifestError::Line { error, .. } => Some(error),
        }
    }
}

impl From<io::Error> for ManifestError {
    fn from(error: io::Error) -> ManifestError {
        ManifestError::Io(error)
    }
}

/// Reads a manifest. SPEC section 4.1.
pub fn read_manifest<R: BufRead>(mut reader: R) -> Result<Vec<Entry>, ManifestError> {
    let mut entries = Vec::new();
    // One buffer for all lines. `read_line` asks the reader for more input
    // only until it has seen the next line feed, so a failure in line 3 is
    // reported before anything after line 3 is requested.
    let mut line = String::new();
    let mut number = 0;
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Ok(entries);
        }
        number += 1;
        // A carriage return is part of the line ending only in front of a
        // line feed. `trim_end` would also remove spaces that belong to a
        // name.
        let text = match line.strip_suffix('\n') {
            Some(text) => text.strip_suffix('\r').unwrap_or(text),
            None => line.as_str(),
        };
        if text.is_empty() {
            continue;
        }
        let entry = text
            .parse()
            .map_err(|error| ManifestError::Line { number, error })?;
        entries.push(entry);
    }
}

/// Writes a manifest. SPEC section 4.2.
pub fn write_manifest<W: Write>(mut out: W, entries: &[Entry]) -> io::Result<()> {
    for entry in entries {
        // `writeln!` formats straight into the writer and uses `write_all`,
        // which repeats short writes and retries interrupted ones.
        writeln!(out, "{entry}")?;
    }
    Ok(())
}

// --------------------------------------------------------------- checking

/// What a check found out about one entry. SPEC section 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The digest is the one in the entry.
    Match,
    /// The digest is this one, and not the one in the entry.
    Changed(u64),
    /// Opening or reading failed with an error of this kind.
    Unreadable(io::ErrorKind),
}

/// Checks one entry on the current thread. The reader is opened, read and
/// dropped here, so it never crosses a thread boundary.
fn check_one<R: Read>(entry: &Entry, open: impl Fn(&str) -> io::Result<R>) -> Status {
    match open(&entry.name).and_then(digest) {
        Ok(actual) if actual == entry.digest => Status::Match,
        Ok(actual) => Status::Changed(actual),
        Err(error) => Status::Unreadable(error.kind()),
    }
}

/// Checks every entry, with up to `workers` threads. SPEC section 5.
pub fn check_all<F, R>(entries: &[Entry], workers: usize, open: F) -> Vec<Status>
where
    F: Fn(&str) -> io::Result<R> + Sync,
    R: Read,
{
    // More threads than entries would have nothing to do.
    let workers = workers.clamp(1, entries.len().max(1));
    // The index of the next entry that nobody has taken. It is the only
    // thing the workers share for writing. The counter protects no other
    // data, so `Relaxed` is enough: each `fetch_add` still returns a
    // different number.
    let next = AtomicUsize::new(0);
    // Each worker takes entries until none is left, and notes the index of
    // every status, because the results must come back in entry order.
    let work = || {
        let mut done = Vec::new();
        loop {
            let index = next.fetch_add(1, Ordering::Relaxed);
            let Some(entry) = entries.get(index) else {
                return done;
            };
            done.push((index, check_one(entry, &open)));
        }
    };

    let mut statuses = vec![Status::Match; entries.len()];
    // Scoped threads may borrow `entries`, `open` and `next`, because the
    // scope does not end before all of them have ended.
    thread::scope(|scope| {
        // `work` captures only shared references, so the closure is `Copy`:
        // every thread gets its own copy of it.
        let helpers: Vec<_> = (1..workers).map(|_| scope.spawn(work)).collect();
        // The calling thread is worker number one.
        let mut done = work();
        for helper in helpers {
            match helper.join() {
                Ok(more) => done.extend(more),
                // A panic in `open` or in a reader is the caller's panic.
                Err(payload) => panic::resume_unwind(payload),
            }
        }
        for (index, status) in done {
            statuses[index] = status;
        }
    });
    statuses
}

/// Checks every entry against the file of that name below `root`. SPEC
/// section 5.
pub fn check_dir(root: &Path, entries: &[Entry], workers: usize) -> Vec<Status> {
    check_all(entries, workers, |name| File::open(root.join(name)))
}

// --------------------------------------------------------------- executor

/// Wakes the thread that sits in `block_on`.
struct Signal {
    thread: Thread,
    /// True when the waker was used since the last poll started.
    woken: AtomicBool,
}

impl Wake for Signal {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // The flag first, then the unpark: the sleeper reads the flag after
        // every return from `park`. `Relaxed` is enough, here and in
        // `block_on`: the flag is the whole message. No other memory is read
        // on the strength of it, so a stronger ordering has nothing to
        // protect.
        self.woken.store(true, Ordering::Relaxed);
        self.thread.unpark();
    }
}

/// Runs a future to its end on the calling thread. SPEC section 6.
pub fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let signal = Arc::new(Signal {
        thread: thread::current(),
        woken: AtomicBool::new(false),
    });
    let waker = Waker::from(Arc::clone(&signal));
    let mut context = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        // Sleep until the flag is set. `park` may return for no reason, and
        // an old waker may unpark this thread; the flag tells real wake-ups
        // from those. A wake-up that came during the poll has already set
        // the flag, so the loop does not sleep at all in that case.
        while !signal.woken.swap(false, Ordering::Relaxed) {
            thread::park();
        }
    }
}

// ------------------------------------------------------- background work

/// The background work panicked. SPEC section 7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panicked;

impl fmt::Display for Panicked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("background work panicked")
    }
}

impl Error for Panicked {}

/// Where the work and the future meet. One lock guards both questions,
/// "is the result there?" and "who waits for it?", so that the answer to
/// one cannot change while the other is being read.
enum Stage<T> {
    /// The work is running. The waker of the most recent poll, if any.
    Running(Option<Waker>),
    /// The work has ended and nobody has fetched the result yet.
    Done(Result<T, Panicked>),
    /// The future has handed the result out.
    Fetched,
}

type Shared<T> = Arc<Mutex<Stage<T>>>;

/// The lock is never held while foreign code runs, so it cannot be
/// poisoned by this module. If it ever is, the data is still consistent.
fn lock<T>(shared: &Shared<T>) -> MutexGuard<'_, Stage<T>> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A future that completes when work on another thread has ended. SPEC
/// section 7.
pub struct Background<T> {
    shared: Shared<T>,
}

impl<T> Future for Background<T> {
    type Output = Result<T, Panicked>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut stage = lock(&self.shared);
        match std::mem::replace(&mut *stage, Stage::Fetched) {
            Stage::Done(result) => Poll::Ready(result),
            Stage::Running(_) => {
                // Always the waker of this poll: an earlier one may belong
                // to a task that no longer owns the future.
                *stage = Stage::Running(Some(cx.waker().clone()));
                Poll::Pending
            }
            Stage::Fetched => panic!("`Background` was polled after it had completed"),
        }
    }
}

/// Starts `work` on a new thread. SPEC section 7.
pub fn in_background<T, W>(work: W) -> Background<T>
where
    W: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let shared: Shared<T> = Arc::new(Mutex::new(Stage::Running(None)));
    let for_thread = Arc::clone(&shared);
    thread::spawn(move || {
        // A panic must not leave the future pending for ever. Nothing of
        // `work` is used after a panic, so the assertion is safe to make.
        let result = panic::catch_unwind(AssertUnwindSafe(work)).map_err(|_| Panicked);
        let waiting = match std::mem::replace(&mut *lock(&for_thread), Stage::Done(result)) {
            Stage::Running(waker) => waker,
            Stage::Done(_) | Stage::Fetched => None,
        };
        // The lock is released before the waker runs: a waker may poll the
        // future at once, on this very thread.
        if let Some(waker) = waiting {
            waker.wake();
        }
    });
    Background { shared }
}

/// The digests of all readers, computed at the same time. SPEC section 8.
pub async fn digest_all<R>(readers: Vec<R>) -> Vec<io::Result<u64>>
where
    R: Read + Send + 'static,
{
    // First start everything: `in_background` begins its work when it is
    // called. `collect` is what makes this loop run to its end before the
    // first `await`.
    let running: Vec<Background<io::Result<u64>>> = readers
        .into_iter()
        .map(|reader| in_background(move || digest(reader)))
        .collect();
    // Then wait for the results in order. While this task waits for one of
    // them, the others go on.
    let mut digests = Vec::with_capacity(running.len());
    for job in running {
        digests.push(match job.await {
            Ok(result) => result,
            Err(Panicked) => Err(io::Error::other(Panicked)),
        });
    }
    digests
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::mpsc;

    #[test]
    fn every_escape_survives_a_round_trip() {
        let entry = Entry {
            digest: 0x1234,
            name: "kettle\\drum\nsolo\r".to_string(),
        };
        let line = entry.to_string();
        assert_eq!(line, "0000000000001234  kettle\\\\drum\\nsolo\\r");
        assert_eq!(line.parse(), Ok(entry));
    }

    #[test]
    fn an_escape_error_counts_bytes_from_the_start_of_the_line() {
        // Two letters of two bytes each in front of the backslash.
        assert_eq!(
            "0000000000000000  çç\\!".parse::<Entry>(),
            Err(EntryError::Escape(22))
        );
    }

    #[test]
    fn a_carriage_return_is_removed_only_before_a_line_feed() {
        let text = b"0000000000000001  a\r\n0000000000000002  b\r";
        let entries = read_manifest(&text[..]).unwrap();
        assert_eq!(entries[0].name, "a");
        assert_eq!(entries[1].name, "b\r");
    }

    #[test]
    fn check_all_starts_no_thread_without_work_for_it() {
        let threads = Mutex::new(HashSet::new());
        let open = |_: &str| {
            threads.lock().unwrap().insert(thread::current().id());
            Ok(io::empty())
        };
        let one = [Entry {
            digest: FNV_OFFSET,
            name: "only".to_string(),
        }];
        assert_eq!(check_all(&one, 8, open), [Status::Match]);
        assert_eq!(
            threads.lock().unwrap().iter().collect::<Vec<_>>(),
            [&thread::current().id()]
        );

        let many: Vec<Entry> = (0..200)
            .map(|i| Entry {
                digest: i,
                name: String::new(),
            })
            .collect();
        assert_eq!(check_all(&many, 3, open).len(), 200);
        assert!(threads.lock().unwrap().len() <= 3);
    }

    #[test]
    fn a_background_job_moves_through_its_stages() {
        let (release, gate) = mpsc::channel::<()>();
        let job = in_background(move || gate.recv().is_ok());
        assert!(matches!(*lock(&job.shared), Stage::Running(None)));
        let shared = Arc::clone(&job.shared);
        release.send(()).unwrap();
        assert_eq!(block_on(job), Ok(true));
        assert!(matches!(*lock(&shared), Stage::Fetched));
    }
}
