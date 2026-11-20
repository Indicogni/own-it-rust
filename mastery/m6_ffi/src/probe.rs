//! Safe access to `csrc/layout_probe.c`. You do not need to edit this file.
//!
//! The probe is C code compiled by build.rs with the same compiler and flags
//! as the library. It answers what Rust cannot find out by itself: how the C
//! compiler lays out `kvtab_stats`, and which values the enumerators have.
//! Under Miri there is no C compiler, so these functions do not exist there.

#![cfg(not(miri))]

use std::ffi::{CStr, c_char, c_int};

// SAFETY: the four prototypes are those in csrc/layout_probe.c.
unsafe extern "C" {
    safe fn m6_probe_stats_size() -> usize;
    safe fn m6_probe_stats_align() -> usize;
    fn m6_probe_stats_offset(field: *const c_char) -> usize;
    fn m6_probe_constant(name: *const c_char) -> c_int;
}

/// `sizeof(kvtab_stats)` according to the C compiler.
pub fn stats_size() -> usize {
    m6_probe_stats_size()
}

/// The alignment of `kvtab_stats` according to the C compiler.
pub fn stats_align() -> usize {
    m6_probe_stats_align()
}

/// `offsetof(kvtab_stats, field)` according to the C compiler.
///
/// # Panics
/// If `kvtab_stats` has no field of that name.
pub fn stats_offset(field: &CStr) -> usize {
    // SAFETY: `field` is a NUL-terminated string; the probe only reads it.
    let offset = unsafe { m6_probe_stats_offset(field.as_ptr()) };
    assert_ne!(offset, usize::MAX, "kvtab_stats has no field {field:?}");
    offset
}

/// The value of an enumerator of kvtab.h, for example `c"KVTAB_ERR_FULL"`.
///
/// # Panics
/// If kvtab.h has no enumerator of that name.
pub fn constant(name: &CStr) -> c_int {
    // SAFETY: `name` is a NUL-terminated string; the probe only reads it.
    let value = unsafe { m6_probe_constant(name.as_ptr()) };
    assert_ne!(value, -1, "kvtab.h has no enumerator {name:?}");
    value
}
