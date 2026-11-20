//! Reference declarations for lesson 01.
//!
//! **Do not read this file before you have finished task A of lesson 01.**
//! It contains the answer. The lesson's `#[deny(clashing_extern_declarations)]`
//! attribute makes the compiler compare your declarations with these, so a
//! wrong translation becomes a compile error. In a real project there is no
//! such second copy: the header is the only truth, and nobody checks.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_long, c_void};

/// `div_t` from `<stdlib.h>`: `typedef struct { int quot; int rem; } div_t;`
///
/// The C standard does not fix the order of the two members. Linux (glibc and
/// musl), macOS and Windows all put `quot` first.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct div_t {
    pub quot: c_int,
    pub rem: c_int,
}

// SAFETY: these are the C standard library's prototypes, translated type by
// type. The C library is linked into every Rust program that uses std.
unsafe extern "C" {
    /// `long strtol(const char *text, char **end, int radix);`
    pub fn strtol(text: *const c_char, end: *mut *mut c_char, radix: c_int) -> c_long;

    /// `div_t div(int numer, int denom);`
    pub fn div(numer: c_int, denom: c_int) -> div_t;

    /// `void *bsearch(const void *key, const void *base, size_t count,
    /// size_t size, int (*compare)(const void *, const void *));`
    pub fn bsearch(
        key: *const c_void,
        base: *const c_void,
        count: usize,
        size: usize,
        compare: unsafe extern "C" fn(*const c_void, *const c_void) -> c_int,
    ) -> *mut c_void;
}
