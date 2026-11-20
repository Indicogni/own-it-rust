//! Raw bindings to `vendor/kvtab/kvtab.h`: the `-sys` layer of this package.
//!
//! **Lesson 03 asks you to write this file yourself.** Do not read on before
//! you have finished that lesson; afterwards, compare your work with it.
//! Lessons 04 to 06 and the capstone use these declarations.
//!
//! A `-sys` layer contains declarations and nothing else: no safe wrappers, no
//! conversions, no policy. Each item below is a translation of one item of the
//! header, in the same order and with the same name, so that a reviewer can
//! put the two files side by side.
//!
//! Who checks this file? Not the Rust compiler: it cannot read C. Three checks
//! exist in this package:
//!
//! - the tests at the bottom ask the C compiler (through `probe.rs`) for the
//!   size, alignment and field offsets of `kvtab_stats` and for the value of
//!   every constant;
//! - `vendor/kvtab/bindgen_reference.rs` is what the `bindgen` tool generated
//!   from the header, kept for comparison (it is not compiled);
//! - under Miri, every call is checked against `src/model.rs`.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_uchar, c_uint, c_void};
use std::marker::{PhantomData, PhantomPinned};

/// `typedef struct kvtab kvtab;` — an opaque table.
///
/// Rust code never has a value of this type, only pointers to one. The fields
/// make sure of that and switch off what the compiler would otherwise assume:
///
/// - the private zero-sized array means that code outside this module cannot
///   construct a `kvtab`, and that the type has size 0 and alignment 1;
/// - `*mut u8` in the marker makes the type `!Send` and `!Sync`. The wrapper
///   decides about threads (lesson 04), not the binding;
/// - `PhantomPinned` makes it `!Unpin`: the address of a C object is part of
///   its identity, so a `kvtab` must never be moved by `mem::swap` or similar.
#[repr(C)]
pub struct kvtab {
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

// `enum kvtab_status`. C functions return these as `int`, so the constants
// are `c_int`. They are constants and not a Rust `enum` on purpose: a Rust
// enum may only ever hold one of its listed values, and a later version of
// the C library may return a new code.

/// Success.
pub const KVTAB_OK: c_int = 0;
/// A required pointer argument was NULL.
pub const KVTAB_ERR_NULL: c_int = 1;
/// The key is not in the table.
pub const KVTAB_ERR_NOT_FOUND: c_int = 2;
/// A new key does not fit: `len == capacity`.
pub const KVTAB_ERR_FULL: c_int = 3;
/// The caller's buffer is too small.
pub const KVTAB_ERR_TOO_SMALL: c_int = 4;
/// `malloc` failed.
pub const KVTAB_ERR_NOMEM: c_int = 5;
/// A visitor asked `kvtab_for_each` to stop.
pub const KVTAB_ERR_STOPPED: c_int = 6;

// `enum kvtab_event`, passed to an observer as `int`.

/// A key was stored or replaced.
pub const KVTAB_EVENT_PUT: c_int = 1;
/// A key was removed.
pub const KVTAB_EVENT_REMOVE: c_int = 2;

/// `struct kvtab_stats`: same fields, same order, same C types.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct kvtab_stats {
    /// Number of entries.
    pub len: usize,
    /// Maximum number of entries.
    pub capacity: usize,
    /// Total size of all stored values.
    pub value_bytes: usize,
    /// Successful `kvtab_put` calls since creation.
    pub puts: c_uint,
    /// 1 when `len == capacity`, else 0.
    pub full: c_uchar,
}

/// `kvtab_visit_fn`. A C function pointer can be NULL, so the Rust type is an
/// `Option`; `None` has the bit pattern of NULL.
pub type kvtab_visit_fn = Option<
    unsafe extern "C" fn(
        key: *const c_char,
        value: *const c_uchar,
        value_len: usize,
        userdata: *mut c_void,
    ) -> c_int,
>;

/// `kvtab_observe_fn`.
pub type kvtab_observe_fn =
    Option<unsafe extern "C" fn(event: c_int, key: *const c_char, userdata: *mut c_void)>;

/// `kvtab_drop_fn`.
pub type kvtab_drop_fn = Option<unsafe extern "C" fn(userdata: *mut c_void)>;

// SAFETY: each declaration below was translated by hand from kvtab.h and
// checked against the header parameter by parameter: symbol names, parameter
// order, C types and const-ness match. No function is marked `safe`, also not
// the ones that cannot go wrong: a `-sys` layer makes no such judgment, so
// that it stays a line-by-line mirror of the header (and of what bindgen
// generates). The safe wrapper is the place for it.
unsafe extern "C" {
    /// `const char *kvtab_version(void);` The result is a static string.
    pub fn kvtab_version() -> *const c_char;

    /// `const char *kvtab_strerror(int status);` The result is a static
    /// string for every `status`, including unknown ones.
    pub fn kvtab_strerror(status: c_int) -> *const c_char;

    /// `kvtab *kvtab_new(size_t capacity);` NULL on failure.
    pub fn kvtab_new(capacity: usize) -> *mut kvtab;

    /// `void kvtab_free(kvtab *t);`
    pub fn kvtab_free(t: *mut kvtab);

    /// `int kvtab_put(kvtab *t, const char *key, const unsigned char *value,
    /// size_t value_len);`
    pub fn kvtab_put(
        t: *mut kvtab,
        key: *const c_char,
        value: *const c_uchar,
        value_len: usize,
    ) -> c_int;

    /// `int kvtab_get(const kvtab *t, const char *key, unsigned char *buf,
    /// size_t *len);`
    pub fn kvtab_get(
        t: *const kvtab,
        key: *const c_char,
        buf: *mut c_uchar,
        len: *mut usize,
    ) -> c_int;

    /// `const unsigned char *kvtab_peek(const kvtab *t, const char *key,
    /// size_t *len);`
    pub fn kvtab_peek(t: *const kvtab, key: *const c_char, len: *mut usize) -> *const c_uchar;

    /// `int kvtab_remove(kvtab *t, const char *key);`
    pub fn kvtab_remove(t: *mut kvtab, key: *const c_char) -> c_int;

    /// `size_t kvtab_len(const kvtab *t);`
    pub fn kvtab_len(t: *const kvtab) -> usize;

    /// `int kvtab_get_stats(const kvtab *t, kvtab_stats *out);`
    pub fn kvtab_get_stats(t: *const kvtab, out: *mut kvtab_stats) -> c_int;

    /// `int kvtab_for_each(const kvtab *t, kvtab_visit_fn visit,
    /// void *userdata);`
    pub fn kvtab_for_each(t: *const kvtab, visit: kvtab_visit_fn, userdata: *mut c_void) -> c_int;

    /// `int kvtab_set_observer(kvtab *t, kvtab_observe_fn observe,
    /// void *userdata, kvtab_drop_fn drop);`
    pub fn kvtab_set_observer(
        t: *mut kvtab,
        observe: kvtab_observe_fn,
        userdata: *mut c_void,
        drop: kvtab_drop_fn,
    ) -> c_int;
}

// The bindings above are checked against the C compiler here. Lesson 03 runs
// the same checks on your bindings.
#[cfg(all(test, not(miri)))]
mod tests {
    use super::*;
    use crate::probe;
    use std::mem::offset_of;

    #[test]
    fn stats_layout_matches_the_c_compiler() {
        assert_eq!(size_of::<kvtab_stats>(), probe::stats_size());
        assert_eq!(align_of::<kvtab_stats>(), probe::stats_align());
        assert_eq!(offset_of!(kvtab_stats, len), probe::stats_offset(c"len"));
        assert_eq!(
            offset_of!(kvtab_stats, capacity),
            probe::stats_offset(c"capacity")
        );
        assert_eq!(
            offset_of!(kvtab_stats, value_bytes),
            probe::stats_offset(c"value_bytes")
        );
        assert_eq!(offset_of!(kvtab_stats, puts), probe::stats_offset(c"puts"));
        assert_eq!(offset_of!(kvtab_stats, full), probe::stats_offset(c"full"));
    }

    #[test]
    fn constants_match_the_header() {
        assert_eq!(KVTAB_OK, probe::constant(c"KVTAB_OK"));
        assert_eq!(KVTAB_ERR_NULL, probe::constant(c"KVTAB_ERR_NULL"));
        assert_eq!(KVTAB_ERR_NOT_FOUND, probe::constant(c"KVTAB_ERR_NOT_FOUND"));
        assert_eq!(KVTAB_ERR_FULL, probe::constant(c"KVTAB_ERR_FULL"));
        assert_eq!(KVTAB_ERR_TOO_SMALL, probe::constant(c"KVTAB_ERR_TOO_SMALL"));
        assert_eq!(KVTAB_ERR_NOMEM, probe::constant(c"KVTAB_ERR_NOMEM"));
        assert_eq!(KVTAB_ERR_STOPPED, probe::constant(c"KVTAB_ERR_STOPPED"));
        assert_eq!(KVTAB_EVENT_PUT, probe::constant(c"KVTAB_EVENT_PUT"));
        assert_eq!(KVTAB_EVENT_REMOVE, probe::constant(c"KVTAB_EVENT_REMOVE"));
    }
}
