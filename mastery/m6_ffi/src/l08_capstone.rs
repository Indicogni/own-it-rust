//! LESSON 08 — Capstone: a Rust library with a C interface, built on a C library
//! Read first: CONCEPTS.md, "Lesson 08". Start after: lessons 01 to 07.
//! Run: cargo test --manifest-path mastery/m6_ffi/Cargo.toml l08_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then draw the call stack of one
//! `m6_stock_for_each` call at the moment when the C visitor runs. Which frames
//! are C, which are Rust, and which of them may a panic pass through?
//! IMPLEMENT the library described below, in this file, and its C header in
//! `include/m6_stock.h`. Nothing is given but the contract: types, helper
//! functions, error handling and every SAFETY argument are your design.
//! EXPLAIN afterward: Write the review that you would want from a colleague. For
//! each exported function: which promises does the header ask of the C caller,
//! which of them can your code check, and what happens when an unchecked one is
//! broken? Then: which single change to the header would break existing C programs
//! without any compiler noticing, and how does `m6_stock_table` limit what you may
//! change inside the library later?
//! MIRI: `cargo +nightly-2026-11-20 miri test --test miri_export l08_` runs the same scenario
//! from Rust against the Rust model of kvtab, with Miri checking every pointer and
//! reporting every leak. The test below that compiles the C program is skipped
//! under Miri.
//!
//! # The contract
//!
//! `m6_stock` is a stock list: item names with quantities. It is a Rust
//! library with a C interface, and it must keep its data in a kvtab table, so
//! one call crosses the boundary twice: C program → your Rust code → C library.
//! Use your `Table` from lessons 04 to 06, or `crate::sys` directly.
//!
//! All names start with `m6_stock`. In the C header:
//!
//! ```c
//! typedef struct m6_stock m6_stock;          /* opaque */
//! typedef int (*m6_stock_visit_fn)(const char *item, int64_t quantity,
//!                                  void *userdata);
//!
//! m6_stock *m6_stock_open(size_t capacity);
//! void m6_stock_close(m6_stock *stock);
//! int m6_stock_adjust(m6_stock *stock, const char *item, int64_t delta,
//!                     int64_t *quantity);
//! int m6_stock_quantity(const m6_stock *stock, const char *item,
//!                       int64_t *quantity);
//! size_t m6_stock_len(const m6_stock *stock);
//! int m6_stock_report(const m6_stock *stock, char *buf, size_t buf_len,
//!                     size_t *needed);
//! int m6_stock_for_each(const m6_stock *stock, m6_stock_visit_fn visit,
//!                       void *userdata);
//! struct kvtab *m6_stock_table(const m6_stock *stock);
//! const char *m6_stock_last_error(void);
//! int m6_stock_debug_panic(void);
//! ```
//!
//! Status codes (functions that return `int` return one of these):
//!
//! | Name | Value | Meaning |
//! | --- | --- | --- |
//! | `M6_STOCK_OK` | 0 | success |
//! | `M6_STOCK_ERR_NULL` | 1 | a required pointer was NULL |
//! | `M6_STOCK_ERR_UTF8` | 2 | an item name is not valid UTF-8 |
//! | `M6_STOCK_ERR_UNKNOWN` | 3 | the item is not in the stock |
//! | `M6_STOCK_ERR_RANGE` | 4 | the quantity would be negative or exceed `INT64_MAX` |
//! | `M6_STOCK_ERR_FULL` | 5 | no room for a new item |
//! | `M6_STOCK_ERR_TOO_SMALL` | 6 | the caller's buffer is too small |
//! | `M6_STOCK_ERR_STOPPED` | 7 | the visitor asked to stop |
//! | `M6_STOCK_ERR_PANIC` | 8 | a panic was caught inside the library |
//! | `M6_STOCK_ERR_INTERNAL` | 9 | the underlying table reported something unexpected |
//!
//! The functions:
//!
//! - **`m6_stock_open(capacity)`** creates a stock for at most `capacity`
//!   items. It returns NULL when `capacity` is 0 or memory is not available.
//! - **`m6_stock_close(stock)`** destroys a stock and its table. NULL is
//!   allowed and does nothing.
//! - **`m6_stock_adjust(stock, item, delta, quantity)`** adds `delta` to the
//!   quantity of `item`. An unknown item starts at 0, so a successful call
//!   creates it (also with `delta == 0`). A quantity must stay in
//!   `0..=INT64_MAX`: otherwise the result is `M6_STOCK_ERR_RANGE`. A new item
//!   that does not fit is `M6_STOCK_ERR_FULL`. On success the new quantity is
//!   written to `*quantity`, unless `quantity` is NULL. On every error nothing
//!   changes: no item is created and `*quantity` is not written.
//! - **`m6_stock_quantity(stock, item, quantity)`** writes the quantity of
//!   `item` to `*quantity`. All three pointers are required.
//! - **`m6_stock_len(stock)`** is the number of items, and 0 for NULL.
//! - **`m6_stock_report(stock, buf, buf_len, needed)`** writes one line
//!   `item=quantity\n` per item, sorted by item name in byte order, and a
//!   final NUL, into `buf`, which has room for `buf_len` bytes. Unless
//!   `needed` is NULL, `*needed` receives the size of the whole report
//!   including the NUL, whether or not it fitted. If it does not fit, `buf`
//!   is not written and the result is `M6_STOCK_ERR_TOO_SMALL`. `buf` may be
//!   NULL when `buf_len` is 0.
//! - **`m6_stock_for_each(stock, visit, userdata)`** calls `visit` once per
//!   item, in the order in which the items were first added, with the name,
//!   the quantity and `userdata`. If `visit` returns a value other than 0,
//!   the walk stops and the result is `M6_STOCK_ERR_STOPPED`. A NULL `visit`
//!   is `M6_STOCK_ERR_NULL`.
//! - **`m6_stock_table(stock)`** returns the underlying kvtab table for
//!   reading, or NULL for a NULL stock. The stock keeps ownership.
//! - **`m6_stock_last_error()`** returns a description of the most recent
//!   failed `m6_stock` call **on the calling thread**, or `""` if none has
//!   failed. It never returns NULL. The pointer stays valid until the next
//!   failed call on the same thread. After `M6_STOCK_ERR_UNKNOWN` the text
//!   contains the item name; after `M6_STOCK_ERR_PANIC` it contains the
//!   panic message. A successful call does not change the text.
//! - **`m6_stock_debug_panic()`** panics inside the library with the message
//!   `deliberate panic`, and returns `M6_STOCK_ERR_PANIC`.
//!
//! Rules for the whole library:
//!
//! - **Storage.** Each item is one kvtab entry: the key is the item name and
//!   the value is the quantity as 8 bytes in little-endian order.
//! - **Panics.** No panic may leave an exported function. Every exported
//!   function that can run code which might panic catches the panic at the
//!   boundary and reports `M6_STOCK_ERR_PANIC` (or NULL from
//!   `m6_stock_open`). `m6_stock_debug_panic` exists to prove it.
//! - **Pointers.** A NULL pointer where one is required is an error code,
//!   never a crash. Everything else that the C caller must guarantee goes
//!   into the header as a comment and into a `# Safety` section.
//!
//! How it is tested: as in lesson 07, the test below builds the static
//! library, compiles `ctests/l08_stock_test.c` against your header, and runs
//! it. That program also includes `kvtab.h` and reads your table directly.

// TODO: write the library here, and its header in include/m6_stock.h.

#[cfg(test)]
mod tests {
    use crate::guess;
    use std::panic::catch_unwind;

    #[test]
    #[cfg_attr(miri, ignore = "builds a static library and runs a C compiler")]
    fn c_program_passes_against_your_header_and_library() {
        let library = crate::harness::staticlib().unwrap_or_else(|error| panic!("{error}"));
        let finished =
            crate::harness::run_c_test("l08_stock_test", library).unwrap_or_else(|error| {
                panic!(
                    "{error}\n\nAn error about an undeclared name means that include/m6_stock.h \
                 does not declare it yet. An error about an undefined symbol or an unresolved \
                 external means that the Rust library does not export it: check the spelling \
                 of the name and that the function has `#[unsafe(no_mangle)]`."
                )
            });
        assert!(
            finished.status.success(),
            "the C test program failed ({}). Each FAIL line names the line of \
             ctests/l08_stock_test.c that did not hold. If the output ends with \
             \"panic in a function that cannot unwind\", a panic reached the boundary \
             and the process was aborted.\n{}",
            finished.status,
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
        // P1. The quantity 258 is stored as `258_i64.to_le_bytes()`. Write
        // the first two bytes and the last byte: [first, second, last].
        let bytes = 258_i64.to_le_bytes();
        let p1: [u8; 3] = guess();
        assert_eq!(p1, [bytes[0], bytes[1], bytes[7]], "P1: little-endian");

        // P2. The range rule of m6_stock_adjust, as one expression. Write the
        // three results.
        let adjust = |current: i64, delta: i64| current.checked_add(delta).filter(|q| *q >= 0);
        let results = [adjust(3, -4), adjust(i64::MAX, 1), adjust(0, 0)];
        let p2: [Option<i64>; 3] = guess();
        assert_eq!(p2, results, "P2: checked arithmetic never panics");

        // P3. A boundary guard wants to put the panic message into the error
        // text. For each panic below, is the payload a `&'static str`?
        // (If not, it is a `String`.) Write [first, second].
        let items = ["axle", "gear"];
        let literal = catch_unwind(|| panic!("P3: expected panic")).unwrap_err();
        let formatted =
            catch_unwind(|| panic!("P3: expected panic with {} items", items.len())).unwrap_err();
        let is_str = [
            literal.downcast_ref::<&'static str>().is_some(),
            formatted.downcast_ref::<&'static str>().is_some(),
        ];
        let p3: [bool; 2] = guess();
        assert_eq!(p3, is_str, "P3: two payload types to handle");
    }
}
