//! LESSON 03 — Bindings by hand: translate a C header and prove the translation
//! Read first: CONCEPTS.md, "Lesson 03". Start after: lesson 02.
//! Run: cargo test --manifest-path mastery/m6_ffi/Cargo.toml l03_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then open `vendor/kvtab/kvtab.h` and read all of it.
//! Consider: the header declares `enum kvtab_status`, but every function returns
//! `int`. Why is that the better choice for a C API, and why must your Rust
//! translation of the status codes not be a Rust `enum`?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Suppose a new version of kvtab adds a field `size_t removes;`
//! in the middle of `kvtab_stats`, and you forget to update your bindings. Which of
//! the checks in this lesson would notice, which would not, and what would
//! `measure` return if none of them existed?
//! MIRI: under Miri your declarations are checked against a Rust model of the
//! library (src/model.rs). The tests that ask the C compiler are skipped there.
//!
//! Your bindings are checked in three ways:
//!
//! 1. The compiler compares every function that you declare with the reference
//!    bindings in `src/sys.rs` (do not look yet). A difference is a compile
//!    error that shows both signatures. If the two signatures look the same
//!    except for the path of `kvtab_stats`, your struct is the difference.
//! 2. The tests ask the C compiler, through `csrc/layout_probe.c`, for the size,
//!    alignment and field offsets of `kvtab_stats` and for the value of every
//!    enumerator, and compare them with yours.
//! 3. The functions that you write below call the library through your
//!    declarations.
//!
//! When you have finished, compare your work with `src/sys.rs` and with
//! `vendor/kvtab/bindgen_reference.rs`, which is what the `bindgen` tool
//! generated from the same header.

// The names in this file are the C names, as in every `-sys` layer.
#![allow(non_camel_case_types)]

use std::ffi::{CStr, c_char, c_int, c_uchar, c_uint, c_void};
use std::mem::MaybeUninit;

/// Supplied: the opaque type `kvtab`. Rust code only ever holds pointers to
/// it. CONCEPTS.md explains its two private fields.
pub use crate::sys::kvtab;

// A. The enumerators of `enum kvtab_status` and `enum kvtab_event`.
// Replace every placeholder value with the value from the header.
// Constants, not a Rust enum: a Rust enum must never hold a value that is not
// one of its variants, and a newer library may return a code that this file
// does not know. An unknown `c_int` is merely an unknown number.
pub const KVTAB_OK: c_int = 0;
pub const KVTAB_ERR_NULL: c_int = 1;
pub const KVTAB_ERR_NOT_FOUND: c_int = 2;
pub const KVTAB_ERR_FULL: c_int = 3;
pub const KVTAB_ERR_TOO_SMALL: c_int = 4;
pub const KVTAB_ERR_NOMEM: c_int = 5;
pub const KVTAB_ERR_STOPPED: c_int = 6;
pub const KVTAB_EVENT_PUT: c_int = 1;
pub const KVTAB_EVENT_REMOVE: c_int = 2;

/// B. `struct kvtab_stats`.
///
/// `repr(C)` gives the fields the order and the padding that the C compiler
/// uses. `size_t` is `usize`, `unsigned int` is `c_uint`, and `unsigned char`
/// is `c_uchar`: not `bool`, because C may store any value from 0 to 255 in
/// it, and a Rust `bool` that holds 2 is undefined behavior.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct kvtab_stats {
    pub len: usize,
    pub capacity: usize,
    pub value_bytes: usize,
    pub puts: c_uint,
    pub full: c_uchar,
}

// C. The three callback types and the twelve functions of the header.
//
// Write a `pub type` alias for each of `kvtab_visit_fn`, `kvtab_observe_fn`
// and `kvtab_drop_fn`. A C function pointer may be NULL, and a Rust function
// pointer may not, so each alias is an `Option<unsafe extern "C" fn(...)>`.
//
// Then declare the twelve functions in the block below, in the order of the
// header. Translate `const T *` to `*const T` and `T *` to `*mut T`. Do not
// mark any function `safe`: a `-sys` layer mirrors the header and makes no
// judgment about which calls cannot go wrong.
pub type kvtab_visit_fn = Option<
    unsafe extern "C" fn(
        key: *const c_char,
        value: *const c_uchar,
        value_len: usize,
        userdata: *mut c_void,
    ) -> c_int,
>;
pub type kvtab_observe_fn =
    Option<unsafe extern "C" fn(event: c_int, key: *const c_char, userdata: *mut c_void)>;
pub type kvtab_drop_fn = Option<unsafe extern "C" fn(userdata: *mut c_void)>;

#[deny(clashing_extern_declarations)]
// SAFETY: translated from vendor/kvtab/kvtab.h, one declaration per
// prototype, in the same order. For each one, the symbol name, the number and
// order of the parameters, every C type and every `const` were compared with
// the header. build.rs links the static library that defines the symbols.
unsafe extern "C" {
    pub fn kvtab_version() -> *const c_char;
    pub fn kvtab_strerror(status: c_int) -> *const c_char;
    pub fn kvtab_new(capacity: usize) -> *mut kvtab;
    pub fn kvtab_free(t: *mut kvtab);
    pub fn kvtab_put(
        t: *mut kvtab,
        key: *const c_char,
        value: *const c_uchar,
        value_len: usize,
    ) -> c_int;
    pub fn kvtab_get(
        t: *const kvtab,
        key: *const c_char,
        buf: *mut c_uchar,
        len: *mut usize,
    ) -> c_int;
    pub fn kvtab_peek(t: *const kvtab, key: *const c_char, len: *mut usize) -> *const c_uchar;
    pub fn kvtab_remove(t: *mut kvtab, key: *const c_char) -> c_int;
    pub fn kvtab_len(t: *const kvtab) -> usize;
    pub fn kvtab_get_stats(t: *const kvtab, out: *mut kvtab_stats) -> c_int;
    pub fn kvtab_for_each(t: *const kvtab, visit: kvtab_visit_fn, userdata: *mut c_void) -> c_int;
    pub fn kvtab_set_observer(
        t: *mut kvtab,
        observe: kvtab_observe_fn,
        userdata: *mut c_void,
        drop: kvtab_drop_fn,
    ) -> c_int;
}

/// D. The names and addresses of the twelve functions that you declared, in
/// the order of the header: `("kvtab_version", kvtab_version as *const ())`
/// and so on.
///
/// A foreign function item is a symbol address that the linker fills in. The
/// test compares each address with the address that the reference bindings
/// get for the same name, which shows that all twelve are declared and that
/// no name has a typing mistake that happens to match another symbol.
pub fn declared() -> [(&'static str, *const ()); 12] {
    [
        ("kvtab_version", kvtab_version as *const ()),
        ("kvtab_strerror", kvtab_strerror as *const ()),
        ("kvtab_new", kvtab_new as *const ()),
        ("kvtab_free", kvtab_free as *const ()),
        ("kvtab_put", kvtab_put as *const ()),
        ("kvtab_get", kvtab_get as *const ()),
        ("kvtab_peek", kvtab_peek as *const ()),
        ("kvtab_remove", kvtab_remove as *const ()),
        ("kvtab_len", kvtab_len as *const ()),
        ("kvtab_get_stats", kvtab_get_stats as *const ()),
        ("kvtab_for_each", kvtab_for_each as *const ()),
        ("kvtab_set_observer", kvtab_set_observer as *const ()),
    ]
}

/// E. The library's description of a status code, as an owned `String`.
/// `kvtab_strerror` returns a static C string for every `int`.
pub fn describe(status: c_int) -> String {
    // SAFETY: kvtab_strerror accepts every int and returns a pointer to a
    // NUL-terminated string with static storage, so the borrow made by
    // `CStr::from_ptr` cannot outlive the string.
    let text = unsafe { CStr::from_ptr(kvtab_strerror(status)) };
    text.to_string_lossy().into_owned()
}

/// F. Fill a table and report its statistics, using only your declarations.
///
/// Create a table for `capacity` entries, put every pair in order, read the
/// statistics with `kvtab_get_stats`, free the table, and return the
/// statistics. Return `None` when the table cannot be created or when any
/// call reports an error. The table must be freed on every path.
///
/// `kvtab_get_stats` fills a struct that the caller provides. Give it a
/// `MaybeUninit<kvtab_stats>` and call `assume_init` only after the function
/// has reported success.
pub fn measure(capacity: usize, pairs: &[(&CStr, &[u8])]) -> Option<kvtab_stats> {
    // SAFETY: kvtab_new accepts every capacity and returns NULL on failure.
    let table = unsafe { kvtab_new(capacity) };
    if table.is_null() {
        return None;
    }
    let mut filled = true;
    for (key, value) in pairs {
        // SAFETY: `table` is a live table that only this function uses. `key`
        // is NUL-terminated. `value` is readable for `value.len()` bytes; for
        // an empty slice the pointer is dangling, and kvtab_put does not read
        // it when the length is 0.
        let status = unsafe { kvtab_put(table, key.as_ptr(), value.as_ptr(), value.len()) };
        if status != KVTAB_OK {
            filled = false;
            break;
        }
    }
    let mut stats = MaybeUninit::<kvtab_stats>::uninit();
    // SAFETY: `table` is live, and `stats.as_mut_ptr()` is valid for a write
    // of one `kvtab_stats`, whose layout the tests compare with the C struct.
    let read = filled && unsafe { kvtab_get_stats(table, stats.as_mut_ptr()) } == KVTAB_OK;
    // SAFETY: `table` came from kvtab_new, is freed exactly once, here, on
    // every path, and is not used afterwards.
    unsafe { kvtab_free(table) };
    // SAFETY: kvtab_get_stats returned KVTAB_OK, so it wrote every field.
    read.then(|| unsafe { stats.assume_init() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::sys;
    use std::mem::offset_of;

    /// Stops a test before it lets C write through a struct of the wrong
    /// size: that would overwrite memory next to it.
    #[track_caller]
    fn require_c_layout() {
        #[cfg(not(miri))]
        let expected = crate::probe::stats_size();
        #[cfg(miri)]
        let expected = size_of::<sys::kvtab_stats>();
        assert_eq!(
            size_of::<kvtab_stats>(),
            expected,
            "kvtab_stats does not have the size of the C struct yet; fix task B \
             before C writes through a pointer to it"
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn a_constants_match_the_header() {
        use crate::probe::constant;
        let pairs = [
            ("KVTAB_OK", KVTAB_OK, constant(c"KVTAB_OK")),
            (
                "KVTAB_ERR_NULL",
                KVTAB_ERR_NULL,
                constant(c"KVTAB_ERR_NULL"),
            ),
            (
                "KVTAB_ERR_NOT_FOUND",
                KVTAB_ERR_NOT_FOUND,
                constant(c"KVTAB_ERR_NOT_FOUND"),
            ),
            (
                "KVTAB_ERR_FULL",
                KVTAB_ERR_FULL,
                constant(c"KVTAB_ERR_FULL"),
            ),
            (
                "KVTAB_ERR_TOO_SMALL",
                KVTAB_ERR_TOO_SMALL,
                constant(c"KVTAB_ERR_TOO_SMALL"),
            ),
            (
                "KVTAB_ERR_NOMEM",
                KVTAB_ERR_NOMEM,
                constant(c"KVTAB_ERR_NOMEM"),
            ),
            (
                "KVTAB_ERR_STOPPED",
                KVTAB_ERR_STOPPED,
                constant(c"KVTAB_ERR_STOPPED"),
            ),
            (
                "KVTAB_EVENT_PUT",
                KVTAB_EVENT_PUT,
                constant(c"KVTAB_EVENT_PUT"),
            ),
            (
                "KVTAB_EVENT_REMOVE",
                KVTAB_EVENT_REMOVE,
                constant(c"KVTAB_EVENT_REMOVE"),
            ),
        ];
        for (name, yours, header) in pairs {
            assert_eq!(
                yours, header,
                "{name}: your constant, then the value in kvtab.h"
            );
        }
    }

    #[test]
    #[cfg(not(miri))]
    fn b_stats_has_the_size_and_alignment_of_the_c_struct() {
        use crate::probe;
        assert_eq!(
            size_of::<kvtab_stats>(),
            probe::stats_size(),
            "size_of::<kvtab_stats>(), then sizeof(kvtab_stats) in C"
        );
        assert_eq!(
            align_of::<kvtab_stats>(),
            probe::stats_align(),
            "align_of::<kvtab_stats>(), then the alignment in C"
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn b_stats_fields_are_where_c_puts_them() {
        use crate::probe::stats_offset;
        let fields = [
            ("len", offset_of!(kvtab_stats, len), stats_offset(c"len")),
            (
                "capacity",
                offset_of!(kvtab_stats, capacity),
                stats_offset(c"capacity"),
            ),
            (
                "value_bytes",
                offset_of!(kvtab_stats, value_bytes),
                stats_offset(c"value_bytes"),
            ),
            ("puts", offset_of!(kvtab_stats, puts), stats_offset(c"puts")),
            ("full", offset_of!(kvtab_stats, full), stats_offset(c"full")),
        ];
        for (name, yours, c) in fields {
            assert_eq!(
                yours, c,
                "offset of `{name}`: in your struct, then in the C struct"
            );
        }
    }

    #[test]
    fn b_stats_matches_the_reference_layout() {
        // The same question as the two tests above, asked of the reference
        // bindings instead of the C compiler, so that it also runs under Miri.
        // No layout test can see whether a field is signed, or how large the
        // last field is: the compiler checks that when you declare
        // kvtab_get_stats in task C.
        assert_eq!(
            (
                size_of::<kvtab_stats>(),
                align_of::<kvtab_stats>(),
                offset_of!(kvtab_stats, puts),
                offset_of!(kvtab_stats, full),
            ),
            (
                size_of::<sys::kvtab_stats>(),
                align_of::<sys::kvtab_stats>(),
                offset_of!(sys::kvtab_stats, puts),
                offset_of!(sys::kvtab_stats, full),
            ),
            "size, alignment, offset of `puts`, offset of `full`: yours, then the reference"
        );
    }

    #[test]
    fn d_all_twelve_functions_are_declared() {
        let names = [
            "kvtab_version",
            "kvtab_strerror",
            "kvtab_new",
            "kvtab_free",
            "kvtab_put",
            "kvtab_get",
            "kvtab_peek",
            "kvtab_remove",
            "kvtab_len",
            "kvtab_get_stats",
            "kvtab_for_each",
            "kvtab_set_observer",
        ];
        let yours = declared();
        assert_eq!(
            yours.map(|(name, _)| name),
            names,
            "the names, in the order of the header"
        );
        // Miri gives a function a new address each time it is asked for one,
        // so the addresses can only be compared in a normal build.
        #[cfg(not(miri))]
        {
            let reference = [
                sys::kvtab_version as *const (),
                sys::kvtab_strerror as *const (),
                sys::kvtab_new as *const (),
                sys::kvtab_free as *const (),
                sys::kvtab_put as *const (),
                sys::kvtab_get as *const (),
                sys::kvtab_peek as *const (),
                sys::kvtab_remove as *const (),
                sys::kvtab_len as *const (),
                sys::kvtab_get_stats as *const (),
                sys::kvtab_for_each as *const (),
                sys::kvtab_set_observer as *const (),
            ];
            for ((name, address), expected) in yours.into_iter().zip(reference) {
                assert!(
                    std::ptr::eq(address, expected),
                    "the entry for {name} is the address of a different function"
                );
            }
        }
    }

    #[test]
    fn e_describe_returns_the_library_text() {
        assert_eq!(describe(0), "success");
        assert_eq!(describe(3), "table is full");
        assert_eq!(describe(6), "stopped by the visitor");
        assert_eq!(
            describe(999),
            "unknown status",
            "kvtab_strerror has an answer for every int"
        );
        assert_eq!(describe(-1), "unknown status");
    }

    #[test]
    fn f_measure_reports_what_c_counted() {
        require_c_layout();
        let stats = measure(3, &[(c"a", b"xy"), (c"b", b""), (c"a", b"wxyz")])
            .expect("three puts into a table for three entries succeed");
        assert_eq!(stats.len, 2, "`a` was stored twice but is one entry");
        assert_eq!(stats.capacity, 3);
        assert_eq!(
            stats.value_bytes, 4,
            "the second value of `a` replaced the first"
        );
        assert_eq!(stats.puts, 3, "three successful kvtab_put calls");
        assert_eq!(stats.full, 0);
    }

    #[test]
    fn f_measure_sees_a_full_table() {
        require_c_layout();
        let stats = measure(2, &[(c"k1", b"1"), (c"k2", b"22")]).expect("both puts succeed");
        assert_eq!(
            (
                stats.len,
                stats.capacity,
                stats.value_bytes,
                stats.puts,
                stats.full
            ),
            (2, 2, 3, 2, 1),
            "len, capacity, value_bytes, puts, full"
        );
    }

    #[test]
    fn f_measure_returns_none_on_failure() {
        require_c_layout();
        assert!(
            measure(0, &[]).is_none(),
            "kvtab_new returns NULL for capacity 0"
        );
        // Under Miri this call also proves that the table is freed on the
        // error path: Miri reports every allocation that is never released.
        assert!(
            measure(1, &[(c"a", b"1"), (c"b", b"2")]).is_none(),
            "the second put returns KVTAB_ERR_FULL"
        );
    }

    #[test]
    fn p_predictions() {
        // The questions are about the reference bindings on a 64-bit target,
        // so they do not depend on your code.

        // P1. struct kvtab_stats has three size_t fields, one unsigned int
        // and one unsigned char. What is its size in bytes?
        let p1: usize = 32;
        assert_eq!(p1, size_of::<sys::kvtab_stats>(), "P1: size with padding");

        // P2. At which offset is the last field, `full`?
        let p2: usize = 28;
        assert_eq!(p2, offset_of!(sys::kvtab_stats, full), "P2: offset of full");

        // P3. `kvtab_visit_fn` is an `Option` of a function pointer. What is
        // its size in bytes?
        let p3: usize = 8;
        assert_eq!(
            p3,
            size_of::<sys::kvtab_visit_fn>(),
            "P3: a nullable function pointer"
        );

        // P4. The opaque type `kvtab`: write (size, alignment).
        let p4: (usize, usize) = (0, 1);
        assert_eq!(
            p4,
            (size_of::<sys::kvtab>(), align_of::<sys::kvtab>()),
            "P4: an opaque type has no layout that Rust knows"
        );
    }
}
