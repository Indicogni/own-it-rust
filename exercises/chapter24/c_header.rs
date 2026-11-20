// Supplied support code for lesson 08. You do not need to open or edit it,
// and reading on gives away task A of that lesson.
//
// A C compiler checks every call against the prototypes in a header file.
// Rust cannot read C headers, so nobody checks an `unsafe extern` block.
// This file is the course's stand-in for the header: a second, correct
// translation of the three prototypes. Lesson 08 turns on the lint
// `clashing_extern_declarations` for the block that you write, and the
// compiler then rejects a declaration that differs from the one here.
// The lesson's predictions also call C through these declarations, so they
// work before you have written your own.

use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    /// `size_t strlen(const char *s);`
    pub fn strlen(s: *const c_char) -> usize;
    /// `int abs(int n);`
    pub fn abs(n: c_int) -> c_int;
    /// `void qsort(void *base, size_t nmemb, size_t size,
    ///             int (*compar)(const void *, const void *));`
    pub fn qsort(
        base: *mut c_void,
        nmemb: usize,
        size: usize,
        compar: unsafe extern "C" fn(*const c_void, *const c_void) -> c_int,
    );
}
