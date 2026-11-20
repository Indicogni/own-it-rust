//! Checkpoint 4: `seal`, a file integrity checker.
//!
//! Read `SPEC.md` first. It is the contract; this file only holds the public
//! names that the tests call, so that the tests compile. Everything else is
//! your design: the fields of `Background`, how threads share their work,
//! how a sleeping thread is woken, private types and helper functions.
//!
//! Rules of the checkpoint (see `../README.md`): closed notes, one sitting,
//! 2 to 4 hours. Do not change the files in `tests/`. Do not change a public
//! name or signature. A struct marked "your design" has a placeholder field;
//! replace it.

// Starter bodies are `todo!()`, so parameters and fields are unused for now.
// Delete this line when you have implemented everything: the compiler's
// warnings are part of your feedback.
#![allow(dead_code, unused_variables)]

use std::error::Error;
use std::fmt;
use std::future::Future;
use std::io::{self, BufRead, Read, Write};
use std::marker::PhantomData;
use std::path::Path;
use std::pin::Pin;
use std::str::FromStr;
use std::task::{Context, Poll};

// ----------------------------------------------------------------- digest

/// The 64-bit FNV-1a hash of everything `reader` gives. SPEC section 2.
pub fn digest<R: Read>(reader: R) -> io::Result<u64> {
    todo!()
}

// ---------------------------------------------------------------- entries

/// One line of a manifest: the digest of a file and its name. SPEC section 3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub digest: u64,
    pub name: String,
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
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
        todo!()
    }
}

impl Error for EntryError {}

impl FromStr for Entry {
    type Err = EntryError;

    fn from_str(line: &str) -> Result<Entry, EntryError> {
        todo!()
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
        todo!()
    }
}

impl Error for ManifestError {}

impl From<io::Error> for ManifestError {
    fn from(error: io::Error) -> ManifestError {
        todo!()
    }
}

/// Reads a manifest. SPEC section 4.1.
pub fn read_manifest<R: BufRead>(reader: R) -> Result<Vec<Entry>, ManifestError> {
    todo!()
}

/// Writes a manifest. SPEC section 4.2.
pub fn write_manifest<W: Write>(out: W, entries: &[Entry]) -> io::Result<()> {
    todo!()
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

/// Checks every entry, with up to `workers` threads. SPEC section 5.
pub fn check_all<F, R>(entries: &[Entry], workers: usize, open: F) -> Vec<Status>
where
    F: Fn(&str) -> io::Result<R> + Sync,
    R: Read,
{
    todo!()
}

/// Checks every entry against the file of that name below `root`. SPEC
/// section 5.
pub fn check_dir(root: &Path, entries: &[Entry], workers: usize) -> Vec<Status> {
    todo!()
}

// --------------------------------------------------------------- executor

/// Runs a future to its end on the calling thread. SPEC section 6.
pub fn block_on<F: Future>(future: F) -> F::Output {
    todo!()
}

// ------------------------------------------------------- background work

/// The background work panicked. SPEC section 7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panicked;

impl fmt::Display for Panicked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for Panicked {}

/// A future that completes when work on another thread has ended. SPEC
/// section 7.
///
/// The fields are your design. Replace the placeholder.
pub struct Background<T> {
    _replace_me: PhantomData<T>,
}

impl<T> Future for Background<T> {
    type Output = Result<T, Panicked>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        todo!()
    }
}

/// Starts `work` on a new thread. SPEC section 7.
pub fn in_background<T, W>(work: W) -> Background<T>
where
    W: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    todo!()
}

/// The digests of all readers, computed at the same time. SPEC section 8.
pub async fn digest_all<R>(readers: Vec<R>) -> Vec<io::Result<u64>>
where
    R: Read + Send + 'static,
{
    todo!()
}
