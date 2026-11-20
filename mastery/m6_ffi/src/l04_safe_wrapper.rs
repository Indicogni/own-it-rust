//! LESSON 04 — A safe wrapper: ownership, error mapping, Drop, and threads
//! Read first: CONCEPTS.md, "Lesson 04". Start after: lesson 03.
//! Run: cargo test --manifest-path mastery/m6_ffi/Cargo.toml l04_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a `Table` holds one raw pointer. The
//! compiler therefore makes it neither `Send` nor `Sync`. Which sentence of
//! `vendor/kvtab/kvtab.h` allows you to add one of the two by hand, and which
//! sentence forbids the other?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Write down the invariant of your `Table` in one sentence.
//! Then go through every method and say whether it relies on the invariant, could
//! break it, or both. Finally explain why `put` takes `&mut self` although the
//! compiler would also accept `&self`.
//! MIRI: this lesson runs under Miri against the Rust model of the library. Miri
//! then reports a table that is never freed, or freed twice.
//!
//! From here on the lessons use the reference bindings in `crate::sys`, and the
//! signatures are only the public surface. The representation of `Table`, its
//! `Drop`, its thread-safety and every SAFETY argument are yours to design.
//! Lessons 05 and 06 add more methods to the `Table` that you write here.

use crate::sys;
use std::ffi::{CStr, c_int};
use std::fmt;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::NonNull;

/// Supplied. Everything that can go wrong in the safe kvtab API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The key is not in the table (`KVTAB_ERR_NOT_FOUND`).
    NotFound,
    /// The table has no room for a new key (`KVTAB_ERR_FULL`).
    Full,
    /// The library could not allocate memory (`KVTAB_ERR_NOMEM`, or a NULL
    /// result of `kvtab_new` for a capacity that is not zero).
    NoMemory,
    /// `Table::new(0)`: a table needs room for at least one entry.
    ZeroCapacity,
    /// A key given as `&str` contains a NUL byte (used in lesson 06).
    InvalidKey,
    /// The caller's buffer is too small: `needed` bytes are required (used in
    /// lesson 06).
    BufferTooSmall { needed: usize },
    /// Any other status. `KVTAB_ERR_NULL` here means a bug in this wrapper;
    /// an unknown number means a newer library than these bindings.
    Unexpected(c_int),
}

impl Error {
    /// A. Turn a kvtab status code into a `Result`.
    ///
    /// `KVTAB_OK` is `Ok(())`. `KVTAB_ERR_NOT_FOUND`, `KVTAB_ERR_FULL` and
    /// `KVTAB_ERR_NOMEM` become the variants of the same name. Every other
    /// number, known or unknown, becomes `Unexpected(number)`: a status code
    /// must never be lost, and never be turned into a wrong variant.
    pub fn check(status: c_int) -> Result<(), Error> {
        todo!("A: map the status code")
    }

    /// A. The kvtab status code that this error stands for, if there is one.
    /// The three errors that belong only to the Rust side return `None`.
    pub fn status(&self) -> Option<c_int> {
        todo!("A: the inverse of check")
    }
}

/// A. Messages:
///
/// - `NotFound`, `Full`, `NoMemory`: exactly the text that `kvtab_strerror`
///   returns for the status code. Do not copy the text into this file; ask
///   the library.
/// - `Unexpected(7)`: `unexpected kvtab status 7: ` followed by the
///   library's text for 7.
/// - `ZeroCapacity`: `capacity must be at least 1`.
/// - `InvalidKey`: `key contains a NUL byte`.
/// - `BufferTooSmall { needed: 9 }`: `buffer too small: 9 bytes needed`.
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("A: write the message")
    }
}

impl std::error::Error for Error {}

/// Supplied. The statistics of a table as plain Rust values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    /// Number of entries.
    pub len: usize,
    /// Maximum number of entries.
    pub capacity: usize,
    /// Total size of all stored values, in bytes.
    pub value_bytes: usize,
    /// Successful `put` calls since the table was created.
    pub puts: u32,
    /// Whether `len == capacity`.
    pub is_full: bool,
}

/// B. An owned kvtab table.
///
/// Design the representation yourself. The requirements:
///
/// - A `Table` is exactly one pointer in size, and so is `Option<Table>`.
/// - A `Table` owns its C table: dropping the `Table` frees the C table
///   exactly once. Nothing in the starter does that; add what is missing.
/// - A `Table` can be moved to another thread, but a `&Table` cannot be
///   shared between threads. Base both decisions on what kvtab.h promises,
///   and write the reason next to the code.
/// - Write the invariant of the type as a comment on its field.
///
/// As soon as the field is a raw pointer or a `NonNull`, the test build of
/// this package stops compiling with E0277: a test of this lesson and a test
/// of lesson 05 move a `Table` to another thread. That error is the
/// compiler's half of the third requirement. Your half is the decision and
/// its reason.
pub struct Table {
    // TODO B: replace this placeholder with your representation.
    _todo: (),
}

impl Table {
    /// B. Creates an empty table for at most `capacity` entries.
    ///
    /// `capacity == 0` is `Error::ZeroCapacity`, decided without calling C.
    /// A NULL result of `kvtab_new` is `Error::NoMemory`.
    pub fn new(capacity: usize) -> Result<Table, Error> {
        todo!("B: kvtab_new, with both failure cases")
    }

    /// B. The raw handle, for calls to `sys` functions that this type does
    /// not wrap. The pointer is valid for as long as the `Table` lives.
    /// Whoever uses it takes over this type's duties: no overlapping calls,
    /// no `kvtab_free`.
    pub fn as_raw(&self) -> *mut sys::kvtab {
        todo!("B: return the pointer")
    }

    /// B. Gives up ownership without freeing: the caller is now responsible
    /// for passing the pointer to `kvtab_free` or to [`Table::from_raw`].
    pub fn into_raw(self) -> *mut sys::kvtab {
        todo!("B: return the pointer and make sure that the table is not freed")
    }

    /// B. Takes ownership of a raw table.
    ///
    /// # Safety
    ///
    /// `raw` must be a pointer that `kvtab_new` or [`Table::into_raw`]
    /// returned, it must not be NULL, the table must not have been freed, and
    /// nothing else may use or free it after this call: the new `Table` is
    /// its only owner.
    pub unsafe fn from_raw(raw: *mut sys::kvtab) -> Table {
        todo!("B: build the Table; the caller has promised the invariant")
    }

    /// B. The number of entries.
    pub fn len(&self) -> usize {
        todo!("B: kvtab_len")
    }

    /// B. Whether the table has no entries.
    pub fn is_empty(&self) -> bool {
        todo!("B: use len")
    }

    /// B. Stores a copy of `value` under `key`. An existing key keeps its
    /// position and gets the new value. Errors: `Full`, `NoMemory`.
    ///
    /// `&mut self` is a decision, not a need of the compiler: `kvtab_put`
    /// frees the old value, and lesson 06 will hand out `&[u8]` borrows of
    /// stored values that are tied to `&self`.
    pub fn put(&mut self, key: &CStr, value: &[u8]) -> Result<(), Error> {
        todo!("B: kvtab_put")
    }

    /// B. Removes `key`. Errors: `NotFound`.
    pub fn remove(&mut self, key: &CStr) -> Result<(), Error> {
        todo!("B: kvtab_remove")
    }

    /// B. A snapshot of the table's counters.
    ///
    /// `kvtab_get_stats` fills a struct that the caller provides, and cannot
    /// fail for arguments that are not NULL. Still, do not read the struct
    /// unless the status says that it was written.
    pub fn stats(&self) -> Stats {
        todo!("B: kvtab_get_stats into a MaybeUninit")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys;
    use crate::{guess, implements, within};
    // Imported here too, so that the tests do not depend on which imports
    // your implementation above still needs.
    use std::ffi::{CStr, c_int, c_void};
    use std::mem::MaybeUninit;
    use std::ptr::NonNull;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// What the library says about a status, for comparison.
    fn library_text(status: c_int) -> String {
        // SAFETY: kvtab_strerror returns a static string for every int.
        unsafe { CStr::from_ptr(sys::kvtab_strerror(status)) }
            .to_str()
            .unwrap()
            .to_string()
    }

    /// A drop function for an observer: adds one to the `AtomicUsize` that
    /// `userdata` points to.
    unsafe extern "C" fn count_free(userdata: *mut c_void) {
        // SAFETY: `watch` passes a pointer to an AtomicUsize that the test
        // keeps alive in an Arc.
        let counter = unsafe { &*userdata.cast::<AtomicUsize>() };
        counter.fetch_add(1, Ordering::SeqCst);
    }

    /// Makes the C table tell us when it is freed: the library calls the
    /// drop function of its observer exactly once, inside `kvtab_free`.
    fn watch(raw: *mut sys::kvtab) -> Arc<AtomicUsize> {
        let frees = Arc::new(AtomicUsize::new(0));
        let userdata = Arc::as_ptr(&frees).cast_mut().cast::<c_void>();
        // SAFETY: `raw` is a live table. `count_free` only reads the pointer
        // as an `&AtomicUsize`, and the caller keeps the Arc alive.
        let status = unsafe { sys::kvtab_set_observer(raw, None, userdata, Some(count_free)) };
        assert_eq!(status, sys::KVTAB_OK);
        frees
    }

    #[test]
    fn a_check_maps_each_status_to_one_error() {
        assert_eq!(Error::check(sys::KVTAB_OK), Ok(()));
        assert_eq!(Error::check(sys::KVTAB_ERR_NOT_FOUND), Err(Error::NotFound));
        assert_eq!(Error::check(sys::KVTAB_ERR_FULL), Err(Error::Full));
        assert_eq!(Error::check(sys::KVTAB_ERR_NOMEM), Err(Error::NoMemory));
    }

    #[test]
    fn a_check_never_loses_a_status_code() {
        for code in [
            sys::KVTAB_ERR_NULL,
            sys::KVTAB_ERR_TOO_SMALL,
            sys::KVTAB_ERR_STOPPED,
            7,
            -1,
            c_int::MAX,
            c_int::MIN,
        ] {
            assert_eq!(
                Error::check(code),
                Err(Error::Unexpected(code)),
                "status {code} has no variant of its own, so it must be kept as a number"
            );
        }
    }

    #[test]
    fn a_status_is_the_inverse_of_check() {
        for code in [2, 3, 5, 1, 4, 6, 99, -5] {
            let error = Error::check(code).unwrap_err();
            assert_eq!(error.status(), Some(code), "{error:?}");
        }
        assert_eq!(Error::ZeroCapacity.status(), None);
        assert_eq!(Error::InvalidKey.status(), None);
        assert_eq!(Error::BufferTooSmall { needed: 3 }.status(), None);
    }

    #[test]
    fn a_display_asks_the_library_for_its_text() {
        assert_eq!(
            Error::NotFound.to_string(),
            library_text(sys::KVTAB_ERR_NOT_FOUND)
        );
        assert_eq!(Error::Full.to_string(), library_text(sys::KVTAB_ERR_FULL));
        assert_eq!(
            Error::NoMemory.to_string(),
            library_text(sys::KVTAB_ERR_NOMEM)
        );
        assert_eq!(
            Error::Unexpected(1).to_string(),
            format!("unexpected kvtab status 1: {}", library_text(1))
        );
        assert_eq!(
            Error::Unexpected(-40).to_string(),
            format!("unexpected kvtab status -40: {}", library_text(-40))
        );
    }

    #[test]
    fn a_display_of_the_rust_side_errors() {
        assert_eq!(
            Error::ZeroCapacity.to_string(),
            "capacity must be at least 1"
        );
        assert_eq!(Error::InvalidKey.to_string(), "key contains a NUL byte");
        assert_eq!(
            Error::BufferTooSmall { needed: 9 }.to_string(),
            "buffer too small: 9 bytes needed"
        );
    }

    #[test]
    fn b_new_rejects_capacity_zero() {
        assert_eq!(Table::new(0).err(), Some(Error::ZeroCapacity));
    }

    #[test]
    fn b_new_reports_a_failed_allocation() {
        // No machine has room for usize::MAX entries: kvtab_new returns NULL.
        assert_eq!(
            Table::new(usize::MAX).err(),
            Some(Error::NoMemory),
            "a NULL result of kvtab_new must become an error, not a Table"
        );
    }

    #[test]
    fn b_put_and_len() {
        let mut table = Table::new(3).unwrap();
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
        table.put(c"apple", b"red").unwrap();
        table.put(c"pear", b"").unwrap();
        assert_eq!(table.len(), 2);
        assert!(!table.is_empty());
        // The C library really has the data.
        // SAFETY: the table is live and no other call is in progress.
        assert_eq!(unsafe { sys::kvtab_len(table.as_raw()) }, 2);
    }

    #[test]
    fn b_put_replaces_an_existing_key() {
        let mut table = Table::new(1).unwrap();
        table.put(c"k", b"first").unwrap();
        table
            .put(c"k", b"second value")
            .expect("replacing a key needs no free slot");
        assert_eq!(table.len(), 1);
        assert_eq!(table.stats().value_bytes, 12);
    }

    #[test]
    fn b_put_reports_a_full_table() {
        let mut table = Table::new(2).unwrap();
        table.put(c"a", b"1").unwrap();
        table.put(c"b", b"2").unwrap();
        assert_eq!(table.put(c"c", b"3"), Err(Error::Full));
        assert_eq!(table.len(), 2, "a failed put changes nothing");
    }

    #[test]
    fn b_remove() {
        let mut table = Table::new(2).unwrap();
        table.put(c"a", b"1").unwrap();
        assert_eq!(table.remove(c"missing"), Err(Error::NotFound));
        assert_eq!(table.remove(c"a"), Ok(()));
        assert_eq!(table.remove(c"a"), Err(Error::NotFound));
        assert!(table.is_empty());
    }

    #[test]
    fn b_stats_are_converted_to_rust_types() {
        let mut table = Table::new(2).unwrap();
        assert_eq!(
            table.stats(),
            Stats {
                len: 0,
                capacity: 2,
                value_bytes: 0,
                puts: 0,
                is_full: false
            }
        );
        table.put(c"a", b"12345").unwrap();
        table.put(c"a", b"123").unwrap();
        table.put(c"b", b"1").unwrap();
        assert_eq!(
            table.stats(),
            Stats {
                len: 2,
                capacity: 2,
                value_bytes: 4,
                puts: 3,
                is_full: true
            }
        );
    }

    #[test]
    fn c_drop_frees_the_table_exactly_once() {
        let table = Table::new(2).unwrap();
        let frees = watch(table.as_raw());
        assert_eq!(frees.load(Ordering::SeqCst), 0);
        drop(table);
        assert_eq!(
            frees.load(Ordering::SeqCst),
            1,
            "dropping a Table must call kvtab_free on its table once; \
             0 means that the C table leaks"
        );
    }

    #[test]
    fn c_into_raw_hands_the_table_over_without_freeing_it() {
        let table = Table::new(2).unwrap();
        let frees = watch(table.as_raw());
        let expected = table.as_raw();
        let raw = table.into_raw();
        assert_eq!(raw, expected, "into_raw returns the same handle");
        assert_eq!(
            frees.load(Ordering::SeqCst),
            0,
            "into_raw must not free the table: the caller owns it now"
        );
        // SAFETY: we own the table since into_raw, and free it once.
        unsafe { sys::kvtab_free(raw) };
        assert_eq!(frees.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn c_from_raw_takes_ownership() {
        // SAFETY: kvtab_new accepts every capacity.
        let raw = unsafe { sys::kvtab_new(4) };
        assert!(!raw.is_null());
        let frees = watch(raw);
        // SAFETY: `raw` is a new table that nothing else owns.
        let mut table = unsafe { Table::from_raw(raw) };
        assert_eq!(
            table.as_raw(),
            raw,
            "from_raw must adopt this table, not make another"
        );
        table.put(c"a", b"1").unwrap();
        assert_eq!(table.len(), 1);
        drop(table);
        assert_eq!(
            frees.load(Ordering::SeqCst),
            1,
            "a Table made by from_raw owns the table and frees it when dropped"
        );
    }

    #[test]
    fn c_technique_a_table_is_one_pointer() {
        assert_eq!(
            size_of::<Table>(),
            size_of::<*mut sys::kvtab>(),
            "a Table is the handle and nothing else"
        );
        assert_eq!(
            size_of::<Option<Table>>(),
            size_of::<*mut sys::kvtab>(),
            "store the handle in a type that knows it is never NULL, \
             so that `None` can use the NULL bit pattern"
        );
    }

    #[test]
    fn d_a_table_is_send_but_not_sync() {
        assert!(
            implements!(Table: Send),
            "kvtab.h allows calls from different threads as long as they do \
             not overlap, so a Table may move to another thread"
        );
        assert!(
            !implements!(Table: Sync),
            "kvtab.h says that calls on one table must never overlap, so two \
             threads must not share a &Table"
        );
    }

    #[test]
    fn d_a_table_works_on_another_thread() {
        let mut table = Table::new(2).unwrap();
        table.put(c"a", b"1").unwrap();
        let len = within(10, "the thread that received the table", move || {
            table.put(c"b", b"2").unwrap();
            table.len()
        });
        assert_eq!(len, 2);
    }

    #[test]
    fn p_predictions() {
        // The questions use the raw `sys` functions on a 64-bit target, so
        // they do not depend on your code.

        // P1. The size in bytes of `Option<NonNull<sys::kvtab>>`.
        let p1: usize = guess();
        assert_eq!(
            p1,
            size_of::<Option<NonNull<sys::kvtab>>>(),
            "P1: NonNull leaves NULL free for None"
        );

        // P2. A table for ONE entry. Which status codes do these three calls
        // return: put "a", put "b", put "a" again?
        // SAFETY: the table is created here, used only here and freed below;
        // all keys are NUL-terminated and the values are readable.
        let (statuses, stats) = unsafe {
            let table = sys::kvtab_new(1);
            let statuses = [
                sys::kvtab_put(table, c"a".as_ptr(), b"1".as_ptr(), 1),
                sys::kvtab_put(table, c"b".as_ptr(), b"2".as_ptr(), 1),
                sys::kvtab_put(table, c"a".as_ptr(), b"3".as_ptr(), 1),
            ];
            let mut stats = MaybeUninit::<sys::kvtab_stats>::uninit();
            assert_eq!(sys::kvtab_get_stats(table, stats.as_mut_ptr()), 0);
            sys::kvtab_free(table);
            (statuses, stats.assume_init())
        };
        let p2: [c_int; 3] = guess();
        assert_eq!(p2, statuses, "P2: full means no room for a NEW key");

        // P3. After those three calls: what are `len` and `puts` in the
        // table's statistics? Write (len, puts).
        let p3: (usize, u32) = guess();
        assert_eq!(
            p3,
            (stats.len, stats.puts),
            "P3: only successful puts count"
        );

        // P4. An observer with a drop function is installed, then replaced by
        // a second one, then the table is freed. How often has the drop
        // function run after the replacement, and how often after the free?
        let first = Arc::new(AtomicUsize::new(0));
        let userdata = Arc::as_ptr(&first).cast_mut().cast::<c_void>();
        // SAFETY: the table is created, used and freed here. Both observers
        // get a pointer to the same AtomicUsize, which outlives the table.
        let counts = unsafe {
            let table = sys::kvtab_new(1);
            sys::kvtab_set_observer(table, None, userdata, Some(count_free));
            sys::kvtab_set_observer(table, None, userdata, Some(count_free));
            let after_replacement = first.load(Ordering::SeqCst);
            sys::kvtab_free(table);
            (after_replacement, first.load(Ordering::SeqCst))
        };
        let p4: (usize, usize) = guess();
        assert_eq!(p4, counts, "P4: each userdata is dropped exactly once");
    }
}
