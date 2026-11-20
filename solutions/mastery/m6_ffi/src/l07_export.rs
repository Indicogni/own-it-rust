//! LESSON 07 — Exporting a Rust library to C: `extern "C"` definitions, a static library, a header
//! Read first: CONCEPTS.md, "Lesson 07". Start after: lessons 04 to 06.
//! Run: cargo test --manifest-path mastery/m6_ffi/Cargo.toml l07_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: until now Rust called C. Now a C
//! program calls Rust. Which of the checks that the Rust compiler did for you in
//! lessons 04 to 06 can it still do when the caller is written in C?
//! IMPLEMENT the library described below, in this file, and its C header in
//! `include/m6_tally.h`. There are no signatures to fill in: every item is yours.
//! EXPLAIN afterward: `m6_tally_top` returns a string that Rust allocated, and the
//! C caller must give it back to `m6_tally_string_free`. Explain what would go
//! wrong if the caller passed it to C's `free` instead, and name two other designs
//! for returning a string to C that avoid a second "free" function.
//! MIRI: `cargo +nightly-2026-11-20 miri test --test miri_export l07_` calls your functions
//! from Rust through the same C declarations, with Miri checking every pointer.
//! The test below that compiles the C program is skipped under Miri.
//!
//! # The contract
//!
//! Write a small library that counts words, with a C interface. All names
//! start with `m6_tally`. In the C header:
//!
//! | C declaration | Meaning |
//! | --- | --- |
//! | `typedef struct m6_tally m6_tally;` | An opaque handle. C never sees its fields. |
//! | `M6_TALLY_OK` = 0, `M6_TALLY_ERR_NULL` = 1, `M6_TALLY_ERR_UTF8` = 2 | Status codes (macros or an enum). |
//! | `uint32_t m6_tally_abi_version(void);` | Returns 1. |
//! | `m6_tally *m6_tally_new(void);` | A new, empty tally. Never NULL. |
//! | `void m6_tally_free(m6_tally *tally);` | Destroys a tally. NULL is allowed and does nothing. |
//! | `int m6_tally_add(m6_tally *tally, const char *text);` | Counts each word of `text`. Words are separated by ASCII white space. |
//! | `uint64_t m6_tally_count(const m6_tally *tally, const char *word);` | How often `word` was counted. |
//! | `char *m6_tally_top(const m6_tally *tally);` | The most frequent word, as a new C string that the caller owns. |
//! | `void m6_tally_string_free(char *text);` | Releases a string from `m6_tally_top`. NULL is allowed. |
//!
//! Details:
//!
//! - `m6_tally_add` returns `M6_TALLY_ERR_NULL` when a pointer is NULL, and
//!   `M6_TALLY_ERR_UTF8` when `text` is not valid UTF-8; in both cases nothing
//!   is counted. Otherwise it returns `M6_TALLY_OK`.
//! - `m6_tally_count` returns 0 when a pointer is NULL, when `word` is not
//!   valid UTF-8, or when the word was never counted.
//! - `m6_tally_top` returns NULL when `tally` is NULL or nothing was counted.
//!   Among words with the same count, the smallest in byte order wins.
//! - No function may panic. Write them so that they cannot (this library is
//!   simple enough); the capstone shows what to do when you cannot be sure.
//!
//! On the Rust side:
//!
//! - Each function is a definition with `#[unsafe(no_mangle)]` and the
//!   `extern "C"` ABI. `no_mangle` is what exports the symbol; make the
//!   functions `pub` as well, so that Rust code and the documentation see
//!   them. Functions that trust a pointer from the caller are
//!   `unsafe extern "C" fn` and have a `# Safety` section.
//! - The handle is a pointer to a Rust type of your design, made with
//!   `Box::into_raw` and destroyed with `Box::from_raw`.
//! - The string of `m6_tally_top` is made with `CString::into_raw` and
//!   released with `CString::from_raw`. Read the documentation of those two
//!   functions before you write the header: the C caller may change the
//!   bytes of the string (the test program does), but there is one thing
//!   that it must not change, and only your header can tell it.
//!
//! How it is tested: `Cargo.toml` asks for the crate type `staticlib` next to
//! `lib`. The test below builds `libm6_ffi.a` (`m6_ffi.lib` on Windows),
//! compiles `ctests/l07_tally_test.c` against your header with the system's C
//! compiler, links the two, and runs the program. Read the C program: it is
//! the precise form of this contract. `src/harness.rs` shows how the build is
//! done.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_int};

/// `M6_TALLY_OK`
pub const M6_TALLY_OK: c_int = 0;
/// `M6_TALLY_ERR_NULL`
pub const M6_TALLY_ERR_NULL: c_int = 1;
/// `M6_TALLY_ERR_UTF8`
pub const M6_TALLY_ERR_UTF8: c_int = 2;

/// The Rust type behind the C handle `m6_tally *`.
///
/// C sees only a pointer to an incomplete struct, so this type needs no
/// `repr(C)` and can change freely: its layout is not part of the ABI.
/// A `BTreeMap` keeps the words in byte order, which makes the tie rule of
/// `m6_tally_top` easy.
pub struct Tally {
    counts: BTreeMap<String, u64>,
}

/// `uint32_t m6_tally_abi_version(void);`
#[unsafe(no_mangle)]
pub extern "C" fn m6_tally_abi_version() -> u32 {
    1
}

/// `m6_tally *m6_tally_new(void);`
///
/// The caller owns the tally and must pass it to [`m6_tally_free`] once.
#[unsafe(no_mangle)]
pub extern "C" fn m6_tally_new() -> *mut Tally {
    // `Box::into_raw` never returns NULL. If the allocation fails, Rust
    // aborts the process; it does not panic.
    Box::into_raw(Box::new(Tally {
        counts: BTreeMap::new(),
    }))
}

/// `void m6_tally_free(m6_tally *tally);`
///
/// # Safety
///
/// `tally` must be NULL or a pointer that [`m6_tally_new`] returned and that
/// has not been freed. It must not be used after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m6_tally_free(tally: *mut Tally) {
    if tally.is_null() {
        return;
    }
    // SAFETY: a non-null `tally` came from `Box::into_raw` in `m6_tally_new`,
    // and the caller promises that this is its only release.
    drop(unsafe { Box::from_raw(tally) });
}

/// `int m6_tally_add(m6_tally *tally, const char *text);`
///
/// # Safety
///
/// `tally` must be NULL or a live tally that no other call is using. `text`
/// must be NULL or point to a NUL-terminated string that is readable and not
/// changed during the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m6_tally_add(tally: *mut Tally, text: *const c_char) -> c_int {
    if tally.is_null() || text.is_null() {
        return M6_TALLY_ERR_NULL;
    }
    // SAFETY: `text` is not NULL, so the caller promises a NUL-terminated
    // string that stays valid for this call. The borrow ends with the call.
    let Ok(text) = unsafe { CStr::from_ptr(text) }.to_str() else {
        return M6_TALLY_ERR_UTF8;
    };
    // SAFETY: `tally` is not NULL, so it is a live tally from `m6_tally_new`,
    // and the caller promises that no other call uses it now.
    let tally = unsafe { &mut *tally };
    for word in text.split_ascii_whitespace() {
        let count = tally.counts.entry(word.to_owned()).or_insert(0);
        // `+= 1` could panic on overflow in a debug build. No panic may
        // leave an `extern "C"` function.
        *count = count.saturating_add(1);
    }
    M6_TALLY_OK
}

/// `uint64_t m6_tally_count(const m6_tally *tally, const char *word);`
///
/// # Safety
///
/// `tally` must be NULL or a live tally that no other call is changing.
/// `word` must be NULL or point to a NUL-terminated string that is readable
/// and not changed during the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m6_tally_count(tally: *const Tally, word: *const c_char) -> u64 {
    if tally.is_null() || word.is_null() {
        return 0;
    }
    // SAFETY: `word` is not NULL, so it is a valid NUL-terminated string.
    let Ok(word) = unsafe { CStr::from_ptr(word) }.to_str() else {
        return 0;
    };
    // SAFETY: `tally` is not NULL, so it is a live tally, and nobody changes
    // it during this call.
    let tally = unsafe { &*tally };
    tally.counts.get(word).copied().unwrap_or(0)
}

/// `char *m6_tally_top(const m6_tally *tally);`
///
/// The result is NULL or a string that the caller owns and must release with
/// [`m6_tally_string_free`].
///
/// # Safety
///
/// `tally` must be NULL or a live tally that no other call is changing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m6_tally_top(tally: *const Tally) -> *mut c_char {
    if tally.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: `tally` is not NULL, so it is a live tally, and nobody changes
    // it during this call.
    let tally = unsafe { &*tally };
    // The map iterates in byte order. `max_by_key` returns the LAST maximum,
    // so walk backwards to get the smallest word among equal counts.
    let top = tally.counts.iter().rev().max_by_key(|(_, count)| **count);
    match top.map(|(word, _)| CString::new(word.as_str())) {
        // `into_raw` hands the allocation to the caller. It must come back
        // through `m6_tally_string_free`: Rust's allocator made it, and C's
        // `free` may belong to a different allocator.
        Some(Ok(word)) => word.into_raw(),
        // A word came out of a C string, so it cannot contain a NUL; if it
        // ever did, NULL is the honest answer, not a panic.
        Some(Err(_)) | None => std::ptr::null_mut(),
    }
}

/// `void m6_tally_string_free(char *text);`
///
/// # Safety
///
/// `text` must be NULL or a pointer that [`m6_tally_top`] returned and that
/// has not been freed. The caller may have changed bytes of the string, but
/// not its length (the position of the NUL).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m6_tally_string_free(text: *mut c_char) {
    if text.is_null() {
        return;
    }
    // SAFETY: a non-null `text` came from `CString::into_raw` in
    // `m6_tally_top`, its length is unchanged, and this is its only release.
    drop(unsafe { CString::from_raw(text) });
}

#[cfg(test)]
mod tests {
    use crate::guess;
    use std::ffi::CString;

    #[test]
    #[cfg_attr(miri, ignore = "builds a static library and runs a C compiler")]
    fn c_program_passes_against_your_header_and_library() {
        let library = crate::harness::staticlib().unwrap_or_else(|error| panic!("{error}"));
        let finished =
            crate::harness::run_c_test("l07_tally_test", library).unwrap_or_else(|error| {
                panic!(
                    "{error}\n\nAn error about an undeclared name means that include/m6_tally.h \
                 does not declare it yet. An error about an undefined symbol or an unresolved \
                 external means that the Rust library does not export it: check the spelling \
                 of the name and that the function has `#[unsafe(no_mangle)]`."
                )
            });
        assert!(
            finished.status.success(),
            "the C test program reported failures (each FAIL line names the line of \
             ctests/l07_tally_test.c that did not hold):\n{}",
            finished.output()
        );
        assert!(
            finished.stdout.contains(" checks, 0 failures"),
            "the C test program did not reach its end:\n{}",
            finished.output()
        );
    }

    #[test]
    fn p_predictions() {
        // P1. `Option<Box<T>>` is how a Rust signature can say "a pointer
        // that owns a T, or NULL". How many bytes is an `Option<Box<u64>>`
        // on a 64-bit target?
        let p1: usize = 8;
        assert_eq!(p1, size_of::<Option<Box<u64>>>(), "P1: NULL is None");

        // P2. `CString::into_raw` gives C a `char *`. How many bytes does the
        // allocation behind it hold for the text "top"?
        let raw = CString::new("top").unwrap().into_raw();
        // SAFETY: `raw` came from `into_raw` just now and is released here.
        let bytes = unsafe { CString::from_raw(raw) }
            .into_bytes_with_nul()
            .len();
        let p2: usize = 4;
        assert_eq!(p2, bytes, "P2: the NUL belongs to the allocation");

        // P3. The contract says that words are separated by ASCII white
        // space. How many words does this text have?
        let words = "pear apple  pear\tfig\n".split_ascii_whitespace().count();
        let p3: usize = 4;
        assert_eq!(
            p3, words,
            "P3: runs of white space separate, they do not create words"
        );
    }
}
