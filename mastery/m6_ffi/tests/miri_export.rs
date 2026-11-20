//! The exported C interfaces of lesson 07 and of the capstone, called from
//! Rust **under Miri**.
//!
//! ```text
//! cargo +nightly-2026-11-20 miri test --manifest-path mastery/m6_ffi/Cargo.toml --test miri_export
//! ```
//!
//! The C test programs in `ctests/` check what your library does. They cannot
//! check how: a C program that frees a string twice or reads a freed handle
//! often still prints the right answer. Miri can. This file declares your
//! functions the way a C header does, in `unsafe extern "C"` blocks, and calls
//! them through those declarations. Under Miri such a call resolves to your
//! `#[unsafe(no_mangle)]` definitions, and Miri checks every pointer on the
//! way: use after free, double free, reads out of bounds, leaks, and a
//! definition whose parameter or return types differ from the declaration.
//!
//! In a normal build this file is empty. Declaring functions that do not exist
//! yet would be a link error, and it would stop the whole package from
//! building before you have written them. Under Miri nothing is linked: until
//! you have written a function, the test that calls it stops with "can't call
//! foreign function `m6_tally_...`", which here means "not defined yet".

#![cfg(miri)]

// The declarations below are resolved by symbol name. This line makes sure
// that the library that defines the symbols is part of the test program.
use m6_ffi as _;

use std::ffi::{CStr, c_char, c_int, c_void};

mod tally {
    use super::*;

    /// The opaque handle, as the C header declares it.
    #[repr(C)]
    pub struct Tally {
        _private: [u8; 0],
    }

    pub const OK: c_int = 0;
    pub const ERR_NULL: c_int = 1;
    pub const ERR_UTF8: c_int = 2;

    // SAFETY: these are the declarations of the contract in
    // src/l07_export.rs. Miri reports a definition that does not match.
    unsafe extern "C" {
        pub fn m6_tally_abi_version() -> u32;
        pub fn m6_tally_new() -> *mut Tally;
        pub fn m6_tally_free(tally: *mut Tally);
        pub fn m6_tally_add(tally: *mut Tally, text: *const c_char) -> c_int;
        pub fn m6_tally_count(tally: *const Tally, word: *const c_char) -> u64;
        pub fn m6_tally_top(tally: *const Tally) -> *mut c_char;
        pub fn m6_tally_string_free(text: *mut c_char);
    }

    #[test]
    fn l07_tally_under_miri() {
        // SAFETY: the calls follow the contract of src/l07_export.rs: the
        // tally is created once, used on this thread only and freed once;
        // every text is NUL-terminated; every string from `m6_tally_top` is
        // given back to `m6_tally_string_free` exactly once.
        unsafe {
            assert_eq!(m6_tally_abi_version(), 1);
            let tally = m6_tally_new();
            assert!(!tally.is_null());
            assert!(
                m6_tally_top(tally).is_null(),
                "an empty tally has no top word"
            );

            assert_eq!(
                m6_tally_add(tally, c"pear apple  pear\tfig\npear apple ".as_ptr()),
                OK
            );
            assert_eq!(m6_tally_count(tally, c"pear".as_ptr()), 3);
            assert_eq!(m6_tally_count(tally, c"apple".as_ptr()), 2);
            assert_eq!(m6_tally_count(tally, c"plum".as_ptr()), 0);

            let top = m6_tally_top(tally);
            assert_eq!(CStr::from_ptr(top), c"pear");
            // The caller owns the string and may change its bytes.
            top.write(b'b' as c_char);
            assert_eq!(CStr::from_ptr(top), c"bear");
            assert_eq!(m6_tally_count(tally, c"pear".as_ptr()), 3);
            m6_tally_string_free(top);

            assert_eq!(m6_tally_add(tally, c"apple".as_ptr()), OK);
            let top = m6_tally_top(tally);
            assert_eq!(
                CStr::from_ptr(top),
                c"apple",
                "a tie goes to the smallest word in byte order"
            );
            m6_tally_string_free(top);

            assert_eq!(
                m6_tally_add(tally, c"plum \xff\xfe plum".as_ptr()),
                ERR_UTF8
            );
            assert_eq!(m6_tally_count(tally, c"plum".as_ptr()), 0);
            assert_eq!(m6_tally_count(tally, c"\xff".as_ptr()), 0);

            assert_eq!(m6_tally_add(std::ptr::null_mut(), c"x".as_ptr()), ERR_NULL);
            assert_eq!(m6_tally_add(tally, std::ptr::null()), ERR_NULL);
            assert_eq!(m6_tally_count(std::ptr::null(), c"pear".as_ptr()), 0);
            assert_eq!(m6_tally_count(tally, std::ptr::null()), 0);
            assert!(m6_tally_top(std::ptr::null()).is_null());
            m6_tally_string_free(std::ptr::null_mut());
            m6_tally_free(std::ptr::null_mut());

            // If this free is missing or wrong, Miri reports a leak or a
            // double free when the test ends.
            m6_tally_free(tally);
        }
    }
}

mod stock {
    use super::*;
    use m6_ffi::sys;

    /// The opaque handle, as the C header declares it.
    #[repr(C)]
    pub struct Stock {
        _private: [u8; 0],
    }

    pub type VisitFn = Option<
        unsafe extern "C" fn(item: *const c_char, quantity: i64, userdata: *mut c_void) -> c_int,
    >;

    pub const OK: c_int = 0;
    pub const ERR_NULL: c_int = 1;
    pub const ERR_UTF8: c_int = 2;
    pub const ERR_UNKNOWN: c_int = 3;
    pub const ERR_RANGE: c_int = 4;
    pub const ERR_FULL: c_int = 5;
    pub const ERR_TOO_SMALL: c_int = 6;
    pub const ERR_STOPPED: c_int = 7;
    pub const ERR_PANIC: c_int = 8;

    // SAFETY: these are the declarations of the contract in
    // src/l08_capstone.rs. Miri reports a definition that does not match.
    unsafe extern "C" {
        pub fn m6_stock_open(capacity: usize) -> *mut Stock;
        pub fn m6_stock_close(stock: *mut Stock);
        pub fn m6_stock_adjust(
            stock: *mut Stock,
            item: *const c_char,
            delta: i64,
            quantity: *mut i64,
        ) -> c_int;
        pub fn m6_stock_quantity(
            stock: *const Stock,
            item: *const c_char,
            quantity: *mut i64,
        ) -> c_int;
        pub fn m6_stock_len(stock: *const Stock) -> usize;
        pub fn m6_stock_report(
            stock: *const Stock,
            buf: *mut c_char,
            buf_len: usize,
            needed: *mut usize,
        ) -> c_int;
        pub fn m6_stock_for_each(
            stock: *const Stock,
            visit: VisitFn,
            userdata: *mut c_void,
        ) -> c_int;
        pub fn m6_stock_table(stock: *const Stock) -> *mut sys::kvtab;
        pub fn m6_stock_last_error() -> *const c_char;
        pub fn m6_stock_debug_panic() -> c_int;
    }

    /// What the visitor collects through `userdata`.
    #[derive(Default)]
    struct Walk {
        items: Vec<(String, i64)>,
        stop_at: usize,
    }

    /// A visitor as a C program would write it, in Rust.
    unsafe extern "C" fn visit(item: *const c_char, quantity: i64, userdata: *mut c_void) -> c_int {
        // SAFETY: the tests pass a pointer to a live `Walk` that nothing
        // else uses during the walk, and the library passes a valid string.
        let (walk, item) = unsafe { (&mut *userdata.cast::<Walk>(), CStr::from_ptr(item)) };
        walk.items
            .push((item.to_str().unwrap().to_string(), quantity));
        c_int::from(walk.stop_at != 0 && walk.items.len() >= walk.stop_at)
    }

    /// The text of the last error on this thread.
    fn last_error() -> String {
        // SAFETY: the function returns a valid C string that stays valid
        // until the next failed call on this thread; it is copied at once.
        unsafe { CStr::from_ptr(m6_stock_last_error()) }
            .to_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn l08_quantities_under_miri() {
        // SAFETY: the calls follow the contract of src/l08_capstone.rs: the
        // stock is opened once, used on this thread only and closed once;
        // names are NUL-terminated; out-pointers point to live variables.
        unsafe {
            assert!(m6_stock_open(0).is_null());
            let stock = m6_stock_open(3);
            assert!(!stock.is_null());
            let mut quantity = -77_i64;

            assert_eq!(
                m6_stock_adjust(stock, c"widget".as_ptr(), 5, &mut quantity),
                OK
            );
            assert_eq!(quantity, 5);
            assert_eq!(
                m6_stock_adjust(stock, c"widget".as_ptr(), -2, &mut quantity),
                OK
            );
            assert_eq!(quantity, 3);

            quantity = -77;
            assert_eq!(
                m6_stock_adjust(stock, c"widget".as_ptr(), -4, &mut quantity),
                ERR_RANGE
            );
            assert_eq!(
                quantity, -77,
                "a failed call must not write the out-parameter"
            );
            assert_eq!(
                m6_stock_adjust(stock, c"ghost".as_ptr(), -1, std::ptr::null_mut()),
                ERR_RANGE
            );
            assert_eq!(
                m6_stock_len(stock),
                1,
                "a refused adjustment creates no item"
            );
            assert_eq!(
                m6_stock_quantity(stock, c"ghost".as_ptr(), &mut quantity),
                ERR_UNKNOWN
            );
            assert!(last_error().contains("ghost"), "{:?}", last_error());

            assert_eq!(
                m6_stock_adjust(stock, c"gear".as_ptr(), 10, std::ptr::null_mut()),
                OK
            );
            assert_eq!(
                m6_stock_adjust(stock, c"axle".as_ptr(), 0, &mut quantity),
                OK
            );
            assert_eq!(quantity, 0);
            assert_eq!(
                m6_stock_adjust(stock, c"nut".as_ptr(), 1, std::ptr::null_mut()),
                ERR_FULL
            );
            assert_eq!(m6_stock_len(stock), 3);

            assert_eq!(
                m6_stock_adjust(stock, c"axle".as_ptr(), i64::MAX, &mut quantity),
                OK
            );
            assert_eq!(quantity, i64::MAX);
            assert_eq!(
                m6_stock_adjust(stock, c"axle".as_ptr(), 1, std::ptr::null_mut()),
                ERR_RANGE
            );

            assert_eq!(
                m6_stock_adjust(
                    std::ptr::null_mut(),
                    c"widget".as_ptr(),
                    1,
                    std::ptr::null_mut()
                ),
                ERR_NULL
            );
            assert_eq!(
                m6_stock_adjust(stock, std::ptr::null(), 1, std::ptr::null_mut()),
                ERR_NULL
            );
            assert_eq!(
                m6_stock_adjust(stock, c"\xff\xfe".as_ptr(), 1, std::ptr::null_mut()),
                ERR_UTF8
            );
            assert_eq!(
                m6_stock_quantity(stock, c"widget".as_ptr(), std::ptr::null_mut()),
                ERR_NULL
            );
            assert_eq!(m6_stock_len(std::ptr::null()), 0);

            m6_stock_close(stock);
            m6_stock_close(std::ptr::null_mut());
        }
    }

    #[test]
    fn l08_report_under_miri() {
        let expected = c"axle=7\ngear=10\nwidget=3\n".to_bytes_with_nul();
        // SAFETY: as in `l08_quantities_under_miri`; every buffer is passed
        // with its true length.
        unsafe {
            let stock = m6_stock_open(4);
            for (item, quantity) in [(c"widget", 3), (c"gear", 10), (c"axle", 7)] {
                assert_eq!(
                    m6_stock_adjust(stock, item.as_ptr(), quantity, std::ptr::null_mut()),
                    OK
                );
            }

            let mut needed = 0_usize;
            assert_eq!(
                m6_stock_report(stock, std::ptr::null_mut(), 0, &mut needed),
                ERR_TOO_SMALL
            );
            assert_eq!(needed, expected.len());

            // One byte too small: Miri would report any write past the end,
            // and the test checks that nothing was written at all.
            let mut small = vec![b'x'; expected.len() - 1];
            assert_eq!(
                m6_stock_report(stock, small.as_mut_ptr().cast(), small.len(), &mut needed),
                ERR_TOO_SMALL
            );
            assert!(small.iter().all(|byte| *byte == b'x'));

            // Exactly the needed size: a write of one byte more is an error
            // that Miri reports.
            let mut exact = vec![b'x'; expected.len()];
            assert_eq!(
                m6_stock_report(
                    stock,
                    exact.as_mut_ptr().cast(),
                    exact.len(),
                    std::ptr::null_mut()
                ),
                OK
            );
            assert_eq!(exact, expected);

            m6_stock_close(stock);
        }
    }

    #[test]
    fn l08_for_each_under_miri() {
        // SAFETY: as in `l08_quantities_under_miri`; `visit` gets a pointer
        // to a `Walk` that lives longer than the call.
        unsafe {
            let stock = m6_stock_open(4);
            for (item, quantity) in [(c"widget", 3), (c"gear", 10), (c"axle", 7), (c"widget", 1)] {
                assert_eq!(
                    m6_stock_adjust(stock, item.as_ptr(), quantity, std::ptr::null_mut()),
                    OK
                );
            }

            let mut walk = Walk::default();
            assert_eq!(
                m6_stock_for_each(stock, Some(visit), (&raw mut walk).cast()),
                OK
            );
            assert_eq!(
                walk.items,
                [
                    ("widget".to_string(), 4),
                    ("gear".to_string(), 10),
                    ("axle".to_string(), 7)
                ],
                "items are visited in the order in which they were first added"
            );

            let mut walk = Walk {
                stop_at: 2,
                ..Walk::default()
            };
            assert_eq!(
                m6_stock_for_each(stock, Some(visit), (&raw mut walk).cast()),
                ERR_STOPPED
            );
            assert_eq!(walk.items.len(), 2);

            assert_eq!(
                m6_stock_for_each(stock, None, std::ptr::null_mut()),
                ERR_NULL
            );
            m6_stock_close(stock);
        }
    }

    #[test]
    fn l08_last_error_belongs_to_one_thread_under_miri() {
        // SAFETY: as in `l08_quantities_under_miri`. Each thread opens, uses
        // and closes a stock of its own. The pointer `mine` is read only
        // before the next failed call on this thread.
        unsafe {
            let stock = m6_stock_open(1);
            let mut quantity = 0_i64;
            assert_eq!(
                m6_stock_quantity(stock, c"here".as_ptr(), &mut quantity),
                ERR_UNKNOWN
            );
            let mine = m6_stock_last_error();
            assert!(CStr::from_ptr(mine).to_str().unwrap().contains("here"));

            let (before, after) = std::thread::spawn(|| {
                let before = last_error();
                // SAFETY: this thread uses only its own stock.
                unsafe {
                    let stock = m6_stock_open(1);
                    let mut quantity = 0_i64;
                    assert_eq!(
                        m6_stock_quantity(stock, c"there".as_ptr(), &mut quantity),
                        ERR_UNKNOWN
                    );
                    m6_stock_close(stock);
                }
                (before, last_error())
            })
            .join()
            .unwrap();
            assert_eq!(
                before, "",
                "a thread on which no call has failed has an empty error text"
            );
            assert!(after.contains("there"), "{after:?}");

            // With one global text for all threads, the failure on the other
            // thread would have replaced (and freed) the string behind
            // `mine`, and Miri would report this read.
            assert!(
                CStr::from_ptr(mine).to_str().unwrap().contains("here"),
                "a failure on another thread must not change this thread's text"
            );
            m6_stock_close(stock);
        }
    }

    #[test]
    fn l08_storage_and_panic_under_miri() {
        // SAFETY: as in `l08_quantities_under_miri`. The table pointer is
        // only read, and only while the stock is open.
        unsafe {
            let stock = m6_stock_open(2);
            assert_eq!(
                m6_stock_adjust(stock, c"widget".as_ptr(), 258, std::ptr::null_mut()),
                OK
            );
            assert!(m6_stock_table(std::ptr::null()).is_null());
            let table = m6_stock_table(stock);
            assert_eq!(sys::kvtab_len(table), 1);
            let mut len = 0_usize;
            let bytes = sys::kvtab_peek(table, c"widget".as_ptr(), &mut len);
            assert_eq!(
                std::slice::from_raw_parts(bytes, len),
                258_i64.to_le_bytes(),
                "a quantity is stored as 8 little-endian bytes"
            );

            assert_eq!(
                m6_stock_debug_panic(),
                ERR_PANIC,
                "a panic must come back as a status code"
            );
            assert!(
                last_error().contains("deliberate panic"),
                "{:?}",
                last_error()
            );
            let mut quantity = 0_i64;
            assert_eq!(
                m6_stock_quantity(stock, c"widget".as_ptr(), &mut quantity),
                OK
            );
            assert_eq!(
                quantity, 258,
                "the library is still usable after a caught panic"
            );
            m6_stock_close(stock);
        }
    }
}
