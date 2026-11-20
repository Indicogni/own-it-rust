//! LESSON 08 — Calling the real C library
//! Run: cargo test --test chapter24 l08_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Who checks that your declaration of strlen matches the C library? Why must a qsort comparator not compute `a - b`?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN: Explain what an `unsafe extern "C"` block promises and who has to keep that promise. For each wrapper, say which precondition of the C function it checks or guarantees before the call.
//! COMPILER LAB (examples/ch24_scratch.rs): Declare strlen with return type c_int in the scratch example and read the warning. Then declare abs with a wrong parameter type: no warning. Explain the difference.
//!
//! Until now the "foreign" functions were written in Rust. The three functions
//! in this lesson are compiled C code in the C library that every Rust program
//! on Linux, macOS and Windows already links, so you need no C compiler.

use std::ffi::{CStr, c_char, c_int, c_void};

/// The type C expects for the last argument of `qsort`:
/// `int (*compar)(const void *, const void *)`.
type Comparator = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;

// A. Declare three functions of the C standard library inside the block below.
// Their C prototypes are:
//
//     size_t strlen(const char *s);
//     int    abs(int n);
//     void   qsort(void *base, size_t nmemb, size_t size,
//                  int (*compar)(const void *, const void *));
//
// Translate them type by type: `size_t` is `usize`, `char` is `c_char`, `int`
// is `c_int`, `void *` is `*mut c_void`, `const T *` is `*const T`, and the
// function pointer is the `Comparator` alias above. Do not mark any of them
// `safe`: each one has a precondition that the compiler cannot check. Write
// `Comparator` itself, not `Option<Comparator>`: `qsort` does not accept a
// null comparator, and the check described below compares the types exactly.
//
// In a real project nobody checks such a block: if it is wrong, the program
// still compiles and links, and then misbehaves. Here the course keeps a
// second, correct declaration (in c_header.rs; do not look yet). The supplied
// `deny` attribute makes the compiler compare your declarations with it, so
// a wrong translation is a compile error that shows the expected signature.
#[deny(clashing_extern_declarations)]
unsafe extern "C" {
    fn strlen(s: *const c_char) -> usize;
    fn abs(n: c_int) -> c_int;
    fn qsort(base: *mut c_void, nmemb: usize, size: usize, compar: Comparator);
}

/// B. The length of a C string in bytes, measured by C's `strlen`.
/// `strlen` reads memory until it finds a NUL byte. A `&CStr` always has one,
/// inside a live buffer, so this safe function leaves no rule for its caller.
/// (A test cannot tell C's `strlen` from a count made in Rust. The task is the
/// foreign call and the SAFETY comment that justifies it.)
fn c_strlen(text: &CStr) -> usize {
    // SAFETY: a CStr points at a live buffer that ends with a NUL byte, so
    // strlen stops inside the buffer. strlen only reads, and keeps nothing.
    unsafe { strlen(text.as_ptr()) }
}

/// C. The absolute value of `n`, computed by C's `abs`.
/// In C, `abs(INT_MIN)` is undefined behavior, because the result does not
/// fit in an `int`. Return `None` for `c_int::MIN` WITHOUT calling C, and
/// `Some(result)` for every other input.
fn checked_c_abs(n: c_int) -> Option<c_int> {
    if n == c_int::MIN {
        return None;
    }
    // SAFETY: abs is defined for every int except INT_MIN, which was rejected
    // above.
    Some(unsafe { abs(n) })
}

/// D. A comparator for `qsort` over `c_int` elements, in ascending order.
/// Return a negative number, zero, or a positive number when `*a` is less
/// than, equal to, or greater than `*b`.
///
/// This function must never panic. A panic cannot leave an `extern "C"`
/// function: the process aborts, and the whole test run stops with "panic in a
/// function that cannot unwind". So do not compute `a - b`, which overflows
/// for values that are far apart.
///
/// # Safety
/// `a` and `b` point to live, aligned, initialized `c_int` values.
unsafe extern "C" fn compare_ints(a: *const c_void, b: *const c_void) -> c_int {
    // SAFETY: the caller promises that both pointers refer to live, aligned,
    // initialized c_int values.
    let (a, b) = unsafe { (*a.cast::<c_int>(), *b.cast::<c_int>()) };
    // Ordering is -1, 0 or 1 as an integer. Comparing cannot overflow, so this
    // function cannot panic.
    a.cmp(&b) as c_int
}

/// E. Sort `items` in place with C's `qsort`. `compare` orders two elements.
/// `qsort` wants a pointer to the first element, the NUMBER of elements, and
/// the size of ONE element in bytes. Return at once when there are fewer than
/// two items: there is nothing to sort, and then you never have to ask what C
/// does with the pointer of an empty slice. (No test can see this early
/// return. It keeps your SAFETY comment short.)
///
/// # Safety
/// `compare` must read both of its arguments as `*const T`, must not write
/// through them, must give consistent answers (a total order), and must not
/// panic.
unsafe fn c_sort<T>(items: &mut [T], compare: Comparator) {
    // Supplied: a compile-time check. qsort cannot sort elements of size zero.
    const { assert!(size_of::<T>() != 0, "qsort cannot sort zero-sized elements") };
    if items.len() < 2 {
        return;
    }
    // SAFETY: `items` is an exclusive borrow of len() initialized elements of
    // size_of::<T>() bytes each, so qsort may read and reorder exactly that
    // memory. Moving a Rust value byte by byte is a valid move. The caller
    // promises that `compare` reads its arguments as T and does not panic.
    unsafe {
        qsort(
            items.as_mut_ptr().cast(),
            items.len(),
            size_of::<T>(),
            compare,
        );
    }
}

// Supplied safe wrapper: this is what a user of your module would call.
fn sort_ints(values: &mut [c_int]) {
    // SAFETY: the elements are c_int, and compare_ints reads its arguments as
    // c_int, only reads them, orders them totally and cannot panic.
    unsafe { c_sort(values, compare_ints) }
}

#[cfg(test)]
mod tests {
    use super::super::c_header;
    use super::*;
    use practice::alloc::count;
    use std::cell::Cell;
    use std::ffi::CString;

    // Miri interprets Rust code and cannot call into the compiled C library
    // (it imitates only a few C functions, such as strlen), so the tests that
    // call C functions are ignored under Miri. The tests of the comparator
    // and of short slices make no C call, so Miri runs them.

    #[test]
    #[cfg_attr(miri, ignore)] // calls the C library
    fn b_strlen_counts_bytes_before_the_nul() {
        assert_eq!(c_strlen(c""), 0);
        assert_eq!(c_strlen(c"abc"), 3);
        assert_eq!(
            c_strlen(c"smörgås"),
            9,
            "strlen counts bytes, not characters"
        );
        assert_eq!(
            c_strlen(c"\xff\xfe"),
            2,
            "a C string is bytes; it does not have to be UTF-8"
        );
        let long = CString::new("x".repeat(10_000)).unwrap();
        assert_eq!(c_strlen(&long), 10_000);
    }

    #[test]
    #[cfg_attr(miri, ignore)] // calls the C library
    fn b_strlen_only_reads() {
        let text = CString::new("measure me").unwrap();
        let (len, allocations) = count(|| c_strlen(&text));
        assert_eq!(len, 10);
        assert_eq!(
            allocations, 0,
            "c_strlen must read the caller's buffer in place, not copy it"
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)] // calls the C library
    fn c_abs_rejects_the_one_undefined_input() {
        assert_eq!(checked_c_abs(-7), Some(7));
        assert_eq!(checked_c_abs(7), Some(7));
        assert_eq!(checked_c_abs(0), Some(0));
        assert_eq!(checked_c_abs(c_int::MAX), Some(c_int::MAX));
        assert_eq!(
            checked_c_abs(c_int::MIN + 1),
            Some(c_int::MAX),
            "the smallest input that C's abs can handle"
        );
        assert_eq!(
            checked_c_abs(c_int::MIN),
            None,
            "abs(INT_MIN) is undefined behavior in C, so C must not be asked"
        );
    }

    fn compare(a: c_int, b: c_int) -> c_int {
        // SAFETY: both pointers refer to live c_int locals of this function.
        unsafe { compare_ints((&raw const a).cast(), (&raw const b).cast()) }
    }

    #[test]
    fn d_comparator_reports_the_sign() {
        assert!(compare(1, 2) < 0, "1 sorts before 2: negative result");
        assert!(compare(2, 1) > 0, "2 sorts after 1: positive result");
        assert_eq!(compare(5, 5), 0, "equal values: zero");
        assert!(compare(-3, 3) < 0);
    }

    // If the test run stops here with "panic in a function that cannot
    // unwind", your comparator overflowed (for example in `a - b`) and
    // panicked inside an extern "C" function.
    #[test]
    fn d_comparator_survives_the_extremes() {
        assert!(
            compare(c_int::MIN, c_int::MAX) < 0,
            "the smallest int sorts before the largest; `a - b` gets this wrong"
        );
        assert!(
            compare(c_int::MAX, c_int::MIN) > 0,
            "the largest int sorts after the smallest; `a - b` gets this wrong"
        );
        assert!(compare(c_int::MIN, 1) < 0);
        assert_eq!(compare(c_int::MIN, c_int::MIN), 0);
    }

    #[test]
    #[cfg_attr(miri, ignore)] // calls the C library
    fn e_ints_are_sorted_ascending() {
        let mut values = [3, -1, 2, -1, 0];
        sort_ints(&mut values);
        assert_eq!(values, [-1, -1, 0, 2, 3]);
        let mut extremes = [c_int::MAX, 0, c_int::MIN, -1];
        sort_ints(&mut extremes);
        assert_eq!(extremes, [c_int::MIN, -1, 0, c_int::MAX]);
        let mut many: Vec<c_int> = (0..1000).map(|i| (i * 7919) % 1000).collect();
        sort_ints(&mut many);
        assert!(
            many.iter().copied().eq(0..1000),
            "1000 distinct values must come out as 0, 1, 2, ..., 999"
        );
    }

    #[test]
    fn e_short_slices_are_left_alone() {
        let mut none: [c_int; 0] = [];
        sort_ints(&mut none);
        let mut one = [42];
        sort_ints(&mut one);
        assert_eq!(one, [42]);
    }

    // An element type that is not an int: two fields with C layout.
    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Entry {
        id: u8,
        score: c_int,
    }

    thread_local! {
        static COMPARISONS: Cell<usize> = const { Cell::new(0) };
    }

    // Orders entries by score, highest first, and counts how often C calls it.
    unsafe extern "C" fn by_score_descending(a: *const c_void, b: *const c_void) -> c_int {
        COMPARISONS.set(COMPARISONS.get().wrapping_add(1));
        // SAFETY: c_sort::<Entry> hands qsort a slice of Entry values.
        let (a, b) = unsafe { (&*a.cast::<Entry>(), &*b.cast::<Entry>()) };
        if a.score > b.score {
            -1
        } else if a.score < b.score {
            1
        } else {
            0
        }
    }

    fn entries() -> [Entry; 5] {
        [(1, 40), (2, -5), (3, 900), (4, 0), (5, 41)].map(|(id, score)| Entry { id, score })
    }

    #[test]
    #[cfg_attr(miri, ignore)] // calls the C library
    fn e_c_sort_passes_the_element_size_and_the_comparator() {
        let mut items = entries();
        let before = COMPARISONS.get();
        // SAFETY: the elements are Entry values, and by_score_descending reads
        // its arguments as Entry, orders them totally and cannot panic.
        unsafe { c_sort(&mut items, by_score_descending) };
        assert!(
            COMPARISONS.get() - before >= 4,
            "qsort must call the comparator that c_sort was given; \
             five items cannot be ordered with fewer than four comparisons"
        );
        assert_eq!(
            items.map(|e| e.id),
            [3, 5, 1, 4, 2],
            "entries ordered by score, highest first. A wrong element size \
             (anything but size_of::<T>()) or a wrong count scrambles them"
        );
        assert_eq!(
            items.map(|e| e.score),
            [900, 41, 40, 0, -5],
            "each id must keep its own score: qsort moves whole elements"
        );
    }

    #[test]
    fn e_c_sort_makes_no_call_for_short_slices() {
        let before = COMPARISONS.get();
        let mut one = [Entry { id: 9, score: 9 }];
        // SAFETY: as in the test above.
        unsafe { c_sort(&mut one, by_score_descending) };
        // SAFETY: as in the test above.
        unsafe { c_sort::<Entry>(&mut [], by_score_descending) };
        assert_eq!(
            COMPARISONS.get() - before,
            0,
            "zero or one item: nothing to compare"
        );
        assert_eq!(one, [Entry { id: 9, score: 9 }]);
    }

    #[test]
    #[cfg_attr(miri, ignore)] // calls the C library
    fn e_sorting_happens_in_place() {
        let mut values: Vec<c_int> = (0..20_000).rev().collect();
        let buffer = values.as_ptr();
        let ((), allocations) = count(|| sort_ints(&mut values));
        assert_eq!(
            allocations, 0,
            "qsort sorts the caller's buffer; no Rust-side copy or scratch buffer is needed"
        );
        assert_eq!(
            values.as_ptr(),
            buffer,
            "the sorted values must be in the same buffer as before"
        );
        assert!(values.iter().copied().eq(0..20_000));
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// These predictions call the real C library through the supplied
    /// declarations, so they work before you have finished task A.
    #[test]
    #[cfg_attr(miri, ignore)] // calls the C library
    fn p_predictions() {
        // P1. What does C's strlen return for "naïve", and how many chars
        // does Rust count in the same text?
        let text = c"naïve";
        // SAFETY: a C string literal is NUL-terminated and lives forever.
        let c_len = unsafe { c_header::strlen(text.as_ptr()) };
        let p1: (usize, usize) = (6, 5);
        assert_eq!(
            p1,
            (c_len, text.to_str().unwrap().chars().count()),
            "P1: bytes versus characters"
        );

        // P2. C's abs for -7, then two Rust answers for the input that C
        // leaves undefined: checked_abs, and "does wrapping_abs return MIN?"
        // SAFETY: -7 is not INT_MIN, the only input that abs cannot handle.
        let c_abs = unsafe { c_header::abs(-7) };
        let p2: (c_int, Option<c_int>, bool) = (7, None, true);
        assert_eq!(
            p2,
            (
                c_abs,
                c_int::MIN.checked_abs(),
                c_int::MIN.wrapping_abs() == c_int::MIN
            ),
            "P2: abs and the one value without a positive partner"
        );

        // P3. qsort is about to sort this array. Which numbers describe it:
        // the element count, the size of one element, and the size of the
        // whole array in bytes? (qsort wants the first two.)
        let items = entries();
        let p3: (usize, usize, usize) = (5, 8, 40);
        assert_eq!(
            p3,
            (items.len(), size_of::<Entry>(), size_of_val(&items)),
            "P3: count, element size, total size"
        );

        // P4. Sort the ids by score, highest first, through the real qsort.
        let mut sorted = entries();
        // SAFETY: base points to 5 Entry values (count and element size match
        // the array), and the comparator reads its arguments as Entry.
        unsafe {
            c_header::qsort(
                sorted.as_mut_ptr().cast(),
                sorted.len(),
                size_of::<Entry>(),
                by_score_descending,
            )
        };
        let p4: [u8; 5] = [3, 5, 1, 4, 2];
        assert_eq!(p4, sorted.map(|e| e.id), "P4: qsort with a comparator");
    }
}
