//! LESSON 01 — The C library again: out-pointers, errno, a struct by value, a pointer into your slice
//! Read first: CONCEPTS.md, "Lesson 01". Start after: Chapter 24.
//! Run: cargo test --manifest-path mastery/m6_ffi/Cargo.toml l01_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: `strtol` returns `LONG_MAX` both for
//! the text "9223372036854775807" and for a number that is far too large. How can a
//! caller tell the two apart? Why is there no symbol called `errno` to link to?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: For each of the three C functions, list every precondition
//! that the C standard gives it. For each one, say whether your wrapper checks it,
//! guarantees it by construction (through a type), or passes it on to its caller.
//! MIRI: this lesson calls compiled C functions, which Miri cannot run. Its tests
//! are skipped under Miri.
//!
//! Chapter 24 called `strlen`, `abs` and `qsort`. The three functions here each
//! add one thing that those did not have:
//!
//! - `strtol` writes a second result through a pointer to a pointer, and reports
//!   overflow in `errno`;
//! - `div` returns a struct by value;
//! - `bsearch` returns a pointer into the array that you passed in.

use std::cell::Cell;
use std::ffi::{CStr, c_char, c_int, c_long, c_void};

/// Supplied. The type C expects for the comparison function of `bsearch`:
/// `int (*compare)(const void *key, const void *element)`.
pub type Comparator = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;

/// Supplied. `ERANGE` from `<errno.h>`: "result too large". The value is 34
/// on Linux, macOS and Windows.
pub const ERANGE: c_int = 34;

// Supplied. In C, `errno` is a macro, not a variable: there is no symbol
// called `errno`. Each C library hides a function behind the macro that
// returns the address of the calling thread's error number, and each calls
// that function something else. `link_name` picks the right symbol; Rust code
// sees one name on every platform.
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_vendor = "apple",
    windows
)))]
compile_error!("lesson 01 does not know the name of the errno function on this platform");

// SAFETY: each `link_name` is the documented function behind the `errno`
// macro of that platform's C library; all of them have the C prototype
// `int *name(void)`.
unsafe extern "C" {
    /// The address of the calling thread's `errno`. The pointer is valid for
    /// reads and writes for as long as the thread lives.
    #[cfg_attr(target_os = "linux", link_name = "__errno_location")]
    #[cfg_attr(
        any(target_vendor = "apple", target_os = "freebsd"),
        link_name = "__error"
    )]
    #[cfg_attr(
        any(target_os = "android", target_os = "netbsd", target_os = "openbsd"),
        link_name = "__errno"
    )]
    #[cfg_attr(windows, link_name = "_errno")]
    pub safe fn errno_location() -> *mut c_int;
}

/// A. Translate `div_t` from `<stdlib.h>`:
///
/// ```c
/// typedef struct { int quot; int rem; } div_t;
/// ```
///
/// `div` returns this struct by value, in registers. The C compiler and the
/// Rust compiler agree on which registers only if the Rust type has the same
/// fields, in the same order, with the C layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DivT {
    pub quot: c_int,
    pub rem: c_int,
}

// A. Declare three functions of the C standard library in the block below.
// Their C prototypes are:
//
//     long   strtol(const char *text, char **end, int radix);
//     div_t  div(int numer, int denom);
//     void  *bsearch(const void *key, const void *base, size_t count,
//                    size_t size,
//                    int (*compare)(const void *, const void *));
//
// Use `DivT` for `div_t` and the `Comparator` alias for the function pointer.
// Do not mark any of them `safe`.
//
// The supplied `deny` attribute makes the compiler compare your declarations
// with a second, correct set in c_reference.rs (do not look yet), so a wrong
// translation is a compile error that shows the expected signature.
#[deny(clashing_extern_declarations)]
// SAFETY: the three prototypes are those of the C standard library, which is
// linked into every program that uses std. `long` is `c_long` (32 bits on
// Windows, 64 bits on 64-bit Linux and macOS), `char **` is a pointer to a
// mutable `*mut c_char`, and `size_t` is `usize`.
unsafe extern "C" {
    fn strtol(text: *const c_char, end: *mut *mut c_char, radix: c_int) -> c_long;
    fn div(numer: c_int, denom: c_int) -> DivT;
    fn bsearch(
        key: *const c_void,
        base: *const c_void,
        count: usize,
        size: usize,
        compare: Comparator,
    ) -> *mut c_void;
}

/// B. The calling thread's `errno`.
pub fn errno() -> c_int {
    // SAFETY: `errno_location` returns a pointer to this thread's error
    // number. It is valid and initialised for as long as the thread lives,
    // and no other thread can write to it.
    unsafe { *errno_location() }
}

/// B. Sets the calling thread's `errno`.
pub fn set_errno(value: c_int) {
    // SAFETY: as in `errno`. The pointer belongs to this thread only, so the
    // write cannot race with a read on another thread.
    unsafe { *errno_location() = value };
}

/// The result of [`parse_long`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parsed {
    /// The number that was read.
    pub value: c_long,
    /// How many bytes of the text belong to the number, counted from the
    /// start of the text. Leading white space counts.
    pub consumed: usize,
}

/// Why [`parse_long`] failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseLongError {
    /// The radix is not 0 and not in `2..=36`.
    BadRadix,
    /// The text does not start with a number (after optional white space).
    NoDigits,
    /// The number does not fit in a C `long`.
    OutOfRange,
}

/// C. Parse the start of `text` as an integer with C's `strtol`.
///
/// `strtol(text, &end, radix)` skips white space, reads an optional sign and
/// as many digits as it can, and stores in `end` a pointer to the first byte
/// that it did not use. Radix 0 means "decide from the prefix": `0x` is
/// hexadecimal, a leading `0` is octal, anything else is decimal.
///
/// The contract of this wrapper:
///
/// - A radix other than 0 or `2..=36` is `BadRadix`, **without calling C**.
///   The C standard does not define `strtol` for such a radix, and the
///   Windows C library ends the process when it gets one.
/// - If no digits were read (`end` is still the start of the text), the
///   result is `NoDigits`.
/// - On overflow `strtol` returns `LONG_MAX` or `LONG_MIN` and sets `errno`
///   to `ERANGE`. That is the only signal: `LONG_MAX` is also a valid result.
///   `strtol` never sets `errno` to 0, so set it to 0 before the call and
///   read it directly after the call. Overflow is `OutOfRange`.
/// - Otherwise the result is the value and the number of bytes consumed,
///   computed from the two pointers.
pub fn parse_long(text: &CStr, radix: u32) -> Result<Parsed, ParseLongError> {
    if radix != 0 && !(2..=36).contains(&radix) {
        return Err(ParseLongError::BadRadix);
    }
    let start = text.as_ptr();
    let mut end: *mut c_char = std::ptr::null_mut();
    set_errno(0);
    // SAFETY: `start` points to a NUL-terminated string that is borrowed for
    // the whole call, and strtol reads no further than the NUL. `&mut end` is
    // a valid place to write one pointer. The radix is 0 or in 2..=36 (so the
    // cast is lossless), which are the values the C standard defines.
    let value = unsafe { strtol(start, &mut end, radix as c_int) };
    // Read errno before any other call into the C library can change it.
    let out_of_range = errno() == ERANGE;
    // SAFETY: strtol stores in `end` a pointer into the same string, at or
    // after `start` and at or before its NUL. So both pointers belong to the
    // same allocation and the distance is not negative.
    let consumed = unsafe { end.cast_const().offset_from_unsigned(start) };
    if consumed == 0 {
        return Err(ParseLongError::NoDigits);
    }
    if out_of_range {
        return Err(ParseLongError::OutOfRange);
    }
    Ok(Parsed { value, consumed })
}

/// D. Quotient and remainder of `numer / denom`, both computed by one call
/// of C's `div`. The quotient is rounded toward zero.
///
/// In C, the behavior of `div` is undefined when the quotient cannot be
/// represented: for `denom == 0`, and for `INT_MIN / -1`. Return `None` for
/// both cases without calling C, and `Some((quotient, remainder))` otherwise.
pub fn divide(numer: c_int, denom: c_int) -> Option<(c_int, c_int)> {
    if denom == 0 || (numer == c_int::MIN && denom == -1) {
        return None;
    }
    // SAFETY: `div` takes no pointers. Its only precondition is that the
    // quotient is representable, which the check above guarantees. `DivT` is
    // `repr(C)` with the fields of `div_t` in the same order, so the struct
    // that C returns by value is read correctly.
    let result = unsafe { div(numer, denom) };
    Some((result.quot, result.rem))
}

thread_local! {
    // Supplied: how often `compare_c_int` has run on this thread. The tests
    // read it to see how many comparisons a search needed.
    static COMPARISONS: Cell<usize> = const { Cell::new(0) };
}

/// Supplied. A comparison function for `bsearch` over `c_int` elements in
/// ascending order. It cannot panic, so nothing unwinds into C.
///
/// # Safety
/// Both pointers must point to valid `c_int` values.
pub unsafe extern "C" fn compare_c_int(key: *const c_void, element: *const c_void) -> c_int {
    COMPARISONS.set(COMPARISONS.get().wrapping_add(1));
    // SAFETY: the caller promises that both pointers point to valid `c_int`s.
    let (key, element) = unsafe { (*key.cast::<c_int>(), *element.cast::<c_int>()) };
    // `key - element` could overflow; `cmp` cannot.
    key.cmp(&element) as c_int
}

/// A slice of `c_int` that is known to be sorted in ascending order.
///
/// `bsearch` has a precondition that a caller can break without any `unsafe`:
/// the array must be sorted the way the comparison function orders it. The C
/// standard does not say what happens otherwise. A safe function must be
/// sound for every argument, so it cannot simply trust its caller. This type
/// checks the order once, when it is created, and `find` then relies on it.
/// The field is private, so `new` is the only way to make a value.
pub struct SortedInts<'a> {
    items: &'a [c_int],
}

impl<'a> SortedInts<'a> {
    /// E. Returns `None` unless `items` is sorted in ascending order.
    /// Equal neighbours are allowed.
    pub fn new(items: &'a [c_int]) -> Option<Self> {
        items.is_sorted().then_some(SortedInts { items })
    }

    /// E. The index of an element equal to `needle`, found by C's `bsearch`
    /// with [`compare_c_int`]. When several elements are equal to `needle`,
    /// any of their indexes is correct.
    ///
    /// `bsearch` returns NULL or a pointer to the element. Turn the pointer
    /// back into an index. Return `None` for an empty slice without calling
    /// C: the pointer of an empty slice is not a pointer to an array, and C
    /// requires a valid pointer even when the count is 0.
    pub fn find(&self, needle: c_int) -> Option<usize> {
        if self.items.is_empty() {
            return None;
        }
        let base = self.items.as_ptr();
        // SAFETY:
        // - `&raw const needle` points to one `c_int` that lives for the call.
        // - `base` points to `items.len()` initialised `c_int`s, each
        //   `size_of::<c_int>()` bytes, and the slice is not empty.
        // - `SortedInts::new` is the only constructor, so the slice is sorted
        //   in the order that `compare_c_int` defines.
        // - `compare_c_int` gets the key and one element, both pointers to
        //   `c_int`, which is what it reads. It cannot unwind.
        // - bsearch keeps no pointer after it returns.
        let found = unsafe {
            bsearch(
                (&raw const needle).cast::<c_void>(),
                base.cast::<c_void>(),
                self.items.len(),
                size_of::<c_int>(),
                compare_c_int,
            )
        };
        if found.is_null() {
            return None;
        }
        // SAFETY: a non-null result points to an element of the array that
        // starts at `base`: same allocation, not before `base`, and a whole
        // number of `c_int`s away from it.
        let index = unsafe {
            found
                .cast_const()
                .cast::<c_int>()
                .offset_from_unsigned(base)
        };
        Some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_reference;
    use crate::guess;

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn b_errno_can_be_written_and_read() {
        set_errno(7);
        assert_eq!(errno(), 7, "errno() must read what set_errno() wrote");
        set_errno(0);
        assert_eq!(errno(), 0);
    }

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn b_errno_belongs_to_one_thread() {
        let here = errno_location() as usize;
        let there = std::thread::spawn(|| {
            set_errno(9);
            (errno(), errno_location() as usize)
        })
        .join()
        .unwrap();
        assert_eq!(there.0, 9);
        assert_ne!(
            here, there.1,
            "each thread has its own errno, at its own address"
        );
    }

    fn ok(value: c_long, consumed: usize) -> Result<Parsed, ParseLongError> {
        Ok(Parsed { value, consumed })
    }

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn c_parses_decimal_numbers() {
        assert_eq!(parse_long(c"42", 10), ok(42, 2));
        assert_eq!(parse_long(c"-7", 10), ok(-7, 2));
        assert_eq!(parse_long(c"+7", 10), ok(7, 2));
        assert_eq!(parse_long(c"0", 10), ok(0, 1));
    }

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn c_behaves_like_strtol_not_like_str_parse() {
        // These are properties of C's strtol. `str::parse` rejects all four.
        assert_eq!(
            parse_long(c"  -12abc", 10),
            ok(-12, 5),
            "strtol skips leading white space and stops at the first non-digit; \
             `consumed` counts from the start of the text"
        );
        assert_eq!(
            parse_long(c"0x1Fz", 16),
            ok(31, 4),
            "radix 16 accepts a 0x prefix"
        );
        assert_eq!(
            parse_long(c"0x1F", 0),
            ok(31, 4),
            "radix 0 reads the prefix"
        );
        assert_eq!(
            parse_long(c"077", 0),
            ok(63, 3),
            "radix 0 reads a leading 0 as octal"
        );
        assert_eq!(parse_long(c"z", 36), ok(35, 1));
        assert_eq!(
            parse_long(c"102", 2),
            ok(2, 2),
            "the digit 2 does not exist in radix 2"
        );
    }

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn c_reports_text_without_a_number() {
        for text in [c"", c"   ", c"abc", c"-", c"+ 1"] {
            assert_eq!(
                parse_long(text, 10),
                Err(ParseLongError::NoDigits),
                "{text:?}: when strtol reads no digits, `end` is the start of the text"
            );
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn c_reports_overflow_through_errno() {
        assert_eq!(
            parse_long(c"99999999999999999999999999", 10),
            Err(ParseLongError::OutOfRange)
        );
        assert_eq!(
            parse_long(c"-99999999999999999999999999", 10),
            Err(ParseLongError::OutOfRange)
        );
    }

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn c_the_largest_long_is_not_an_overflow() {
        // LONG_MAX is 2^31 - 1 on Windows and 2^63 - 1 on 64-bit Linux and macOS.
        for value in [c_long::MAX, c_long::MIN] {
            let text = std::ffi::CString::new(value.to_string()).unwrap();
            let digits = text.as_bytes().len();
            assert_eq!(
                parse_long(&text, 10),
                ok(value, digits),
                "{value} fits in a long. Only errno == ERANGE means overflow, \
                 not the returned value"
            );
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn c_a_stale_errno_is_not_an_overflow() {
        set_errno(ERANGE);
        assert_eq!(
            parse_long(c"5", 10),
            ok(5, 1),
            "strtol never clears errno: set it to 0 before the call"
        );
    }

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn c_rejects_a_bad_radix_before_calling_c() {
        for radix in [1, 37, 100, u32::MAX] {
            assert_eq!(parse_long(c"11", radix), Err(ParseLongError::BadRadix));
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn d_divides_toward_zero() {
        assert_eq!(divide(7, 2), Some((3, 1)));
        assert_eq!(divide(-7, 2), Some((-3, -1)));
        assert_eq!(divide(7, -2), Some((-3, 1)));
        assert_eq!(divide(0, 5), Some((0, 0)));
        assert_eq!(
            divide(c_int::MIN, 1),
            Some((c_int::MIN, 0)),
            "only INT_MIN / -1 is undefined, not every division of INT_MIN"
        );
        assert_eq!(divide(c_int::MAX, -1), Some((-c_int::MAX, 0)));
    }

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn d_refuses_the_two_undefined_divisions() {
        assert_eq!(divide(1, 0), None);
        assert_eq!(divide(0, 0), None);
        assert_eq!(
            divide(c_int::MIN, -1),
            None,
            "the quotient 2^31 does not fit in an int"
        );
    }

    #[test]
    fn a_div_t_has_the_c_layout() {
        assert_eq!(
            size_of::<DivT>(),
            2 * size_of::<c_int>(),
            "DivT needs the two int fields of div_t"
        );
        assert_eq!(align_of::<DivT>(), align_of::<c_int>());
    }

    #[test]
    fn e_new_accepts_only_sorted_slices() {
        assert!(SortedInts::new(&[]).is_some());
        assert!(SortedInts::new(&[5]).is_some());
        assert!(SortedInts::new(&[-3, 0, 0, 9]).is_some());
        assert!(
            SortedInts::new(&[1, 3, 2]).is_none(),
            "find relies on the order, so new must refuse an unsorted slice"
        );
        assert!(SortedInts::new(&[2, 1]).is_none());
    }

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn e_finds_the_index_of_an_element() {
        let items = [-20, -3, 0, 4, 4, 9, 100];
        let sorted = SortedInts::new(&items).unwrap();
        assert_eq!(sorted.find(-20), Some(0));
        assert_eq!(sorted.find(0), Some(2));
        assert_eq!(sorted.find(100), Some(6));
        let four = sorted.find(4).expect("4 is in the slice");
        assert!(
            four == 3 || four == 4,
            "index {four} does not hold a 4; compute the index from the returned pointer"
        );
        assert_eq!(sorted.find(5), None);
        assert_eq!(sorted.find(c_int::MIN), None);
        assert_eq!(sorted.find(c_int::MAX), None);
    }

    #[test]
    fn e_an_empty_slice_never_reaches_c() {
        let before = COMPARISONS.get();
        assert_eq!(SortedInts::new(&[]).unwrap().find(1), None);
        assert_eq!(COMPARISONS.get(), before);
    }

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn e_technique_the_search_is_done_by_bsearch() {
        let items: Vec<c_int> = (0..1024).map(|n| n * 3).collect();
        let sorted = SortedInts::new(&items).unwrap();
        let before = COMPARISONS.get();
        assert_eq!(sorted.find(3 * 700), Some(700));
        let comparisons = COMPARISONS.get() - before;
        assert!(
            comparisons >= 1,
            "find must call C's bsearch with compare_c_int; the comparison \
             function was never called"
        );
        assert!(
            comparisons <= 12,
            "a binary search of 1024 elements needs at most 11 comparisons, \
             but {comparisons} were made"
        );
    }

    #[test]
    #[cfg_attr(miri, ignore = "calls compiled C code, which Miri cannot run")]
    fn p_predictions() {
        // These predictions call the C library through the reference
        // declarations, so they do not depend on your code.

        // P1. strtol("  -12abc", &end, 10): which value does it return, and
        // how many bytes lie between the start of the text and `end`?
        let text = c"  -12abc";
        let mut end: *mut c_char = std::ptr::null_mut();
        // SAFETY: `text` is NUL-terminated and `end` is a valid out-pointer.
        let value = unsafe { c_reference::strtol(text.as_ptr(), &mut end, 10) };
        // SAFETY: strtol sets `end` to a position inside `text`.
        let consumed = unsafe { end.cast_const().offset_from_unsigned(text.as_ptr()) };
        let p1: (c_long, usize) = (-12, 5);
        assert_eq!(p1, (value, consumed), "P1: strtol and its end pointer");

        // P2. With radix 0, strtol decides from the prefix. What are
        // strtol("0x10", NULL, 0) and strtol("010", NULL, 0)?
        // SAFETY: both texts are NUL-terminated; a NULL end pointer is allowed.
        let (hex, octal) = unsafe {
            (
                c_reference::strtol(c"0x10".as_ptr(), std::ptr::null_mut(), 0),
                c_reference::strtol(c"010".as_ptr(), std::ptr::null_mut(), 0),
            )
        };
        let p2: [c_long; 2] = [16, 8];
        assert_eq!(p2, [hex, octal], "P2: radix 0");

        // P3. div(-7, 2) returns { quot, rem }. Write (quot, rem).
        // SAFETY: the quotient is representable.
        let result = unsafe { c_reference::div(-7, 2) };
        let p3: (c_int, c_int) = (-3, -1);
        assert_eq!(p3, (result.quot, result.rem), "P3: div rounds toward zero");

        // P4. A number with 26 digits does not fit in a long on any platform.
        // After the call below: is errno equal to ERANGE, and is the returned
        // value equal to LONG_MAX? Write (errno_is_erange, value_is_long_max).
        // SAFETY: the pointer returned by errno_location is valid on this thread.
        unsafe { *errno_location() = 0 };
        // SAFETY: the text is NUL-terminated; a NULL end pointer is allowed.
        let huge = unsafe {
            c_reference::strtol(
                c"99999999999999999999999999".as_ptr(),
                std::ptr::null_mut(),
                10,
            )
        };
        // SAFETY: as above.
        let error = unsafe { *errno_location() };
        let p4: (bool, bool) = (true, true);
        assert_eq!(
            p4,
            (error == ERANGE, huge == c_long::MAX),
            "P4: how strtol reports overflow"
        );
    }
}
