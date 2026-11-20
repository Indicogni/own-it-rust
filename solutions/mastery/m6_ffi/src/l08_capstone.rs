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

use crate::l04_safe_wrapper::{Error, Table};
use crate::sys;
use std::any::Any;
use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::ops::ControlFlow;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// `M6_STOCK_OK`
pub const M6_STOCK_OK: c_int = 0;
/// `M6_STOCK_ERR_NULL`
pub const M6_STOCK_ERR_NULL: c_int = 1;
/// `M6_STOCK_ERR_UTF8`
pub const M6_STOCK_ERR_UTF8: c_int = 2;
/// `M6_STOCK_ERR_UNKNOWN`
pub const M6_STOCK_ERR_UNKNOWN: c_int = 3;
/// `M6_STOCK_ERR_RANGE`
pub const M6_STOCK_ERR_RANGE: c_int = 4;
/// `M6_STOCK_ERR_FULL`
pub const M6_STOCK_ERR_FULL: c_int = 5;
/// `M6_STOCK_ERR_TOO_SMALL`
pub const M6_STOCK_ERR_TOO_SMALL: c_int = 6;
/// `M6_STOCK_ERR_STOPPED`
pub const M6_STOCK_ERR_STOPPED: c_int = 7;
/// `M6_STOCK_ERR_PANIC`
pub const M6_STOCK_ERR_PANIC: c_int = 8;
/// `M6_STOCK_ERR_INTERNAL`
pub const M6_STOCK_ERR_INTERNAL: c_int = 9;

/// `m6_stock_visit_fn`. C may pass NULL, so the Rust type is an `Option`.
pub type StockVisitFn = Option<
    unsafe extern "C" fn(item: *const c_char, quantity: i64, userdata: *mut c_void) -> c_int,
>;

/// The Rust type behind the C handle `m6_stock *`. C sees only a pointer to
/// an incomplete struct, so the layout is free to change.
pub struct Stock {
    table: Table,
}

/// A failure inside the library: the status for the C caller, and the text
/// for `m6_stock_last_error`.
struct Failure {
    status: c_int,
    message: String,
}

impl Failure {
    fn new(status: c_int, message: impl Into<String>) -> Failure {
        Failure {
            status,
            message: message.into(),
        }
    }

    fn null(what: &str) -> Failure {
        Failure::new(M6_STOCK_ERR_NULL, format!("{what} is NULL"))
    }

    /// An error of the underlying table that this library did not expect.
    fn internal(error: Error) -> Failure {
        Failure::new(M6_STOCK_ERR_INTERNAL, format!("kvtab: {error}"))
    }
}

thread_local! {
    /// The text of the last failure on this thread. A thread-local needs no
    /// lock, and one thread's errors never overwrite another's, like errno.
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::default());
}

fn set_last_error(message: &str) {
    // A C string cannot contain NUL; replace any, so that this cannot fail.
    let message = CString::new(message.replace('\0', "?")).unwrap_or_default();
    // `try_with` instead of `with`: during thread shutdown the slot may be
    // gone, and a panic here would be a panic at the boundary.
    let _ = LAST_ERROR.try_with(|slot| *slot.borrow_mut() = message);
}

/// The text of a panic payload. `panic!("literal")` carries a `&str`, and
/// `panic!("{x}")` a `String`.
fn panic_text(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("a panic without a text message")
}

/// Records a caught panic for `m6_stock_last_error` and disposes of its
/// payload.
///
/// This code runs after `catch_unwind` has returned, so nothing protects it:
/// a panic here would reach the `extern "C"` frame and abort the process.
/// Formatting a `&str` and storing a `CString` do not panic. The payload is
/// different: it is a value of any type, and dropping it runs that type's
/// destructor, which is code that this library did not write. So it is
/// dropped inside a `catch_unwind` of its own. (`mem::forget(payload)` would
/// also be safe, but it leaks, and Miri reports the leak.)
fn record_panic(payload: Box<dyn Any + Send>) {
    set_last_error(&format!("panic: {}", panic_text(payload.as_ref())));
    if let Err(second) = catch_unwind(AssertUnwindSafe(move || drop(payload))) {
        // A destructor that panics while its value is being dropped: give
        // up on that value rather than try again.
        std::mem::forget(second);
    }
}

/// The boundary guard: runs the body of an exported function, turns a
/// `Failure` into its status code and a panic into `M6_STOCK_ERR_PANIC`, and
/// records the message for `m6_stock_last_error`.
///
/// AssertUnwindSafe: after a caught panic the caller only gets a status
/// code. The stock itself stays consistent, because every change to it is a
/// single kvtab call, and kvtab does not call back into code that can panic.
fn guard(body: impl FnOnce() -> Result<(), Failure>) -> c_int {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(())) => M6_STOCK_OK,
        Ok(Err(failure)) => {
            set_last_error(&failure.message);
            failure.status
        }
        Err(payload) => {
            record_panic(payload);
            M6_STOCK_ERR_PANIC
        }
    }
}

/// Borrows an item name from C and checks that it is UTF-8.
///
/// # Safety
/// `item` must be NULL or point to a NUL-terminated string that is readable
/// and unchanged for `'a`.
unsafe fn item_name<'a>(item: *const c_char) -> Result<(&'a CStr, &'a str), Failure> {
    if item.is_null() {
        return Err(Failure::null("item"));
    }
    // SAFETY: `item` is not NULL, so the caller promises a NUL-terminated
    // string that lives for `'a`.
    let item = unsafe { CStr::from_ptr(item) };
    let name = item
        .to_str()
        .map_err(|_| Failure::new(M6_STOCK_ERR_UTF8, "item name is not valid UTF-8"))?;
    Ok((item, name))
}

/// Decodes a stored quantity: 8 bytes, little-endian.
fn decode(value: &[u8]) -> Result<i64, Failure> {
    let bytes: [u8; 8] = value.try_into().map_err(|_| {
        Failure::new(
            M6_STOCK_ERR_INTERNAL,
            format!("a stored quantity has {} bytes instead of 8", value.len()),
        )
    })?;
    Ok(i64::from_le_bytes(bytes))
}

/// The quantity of `item`, or `None` when it is not in the stock.
fn lookup(table: &Table, item: &CStr) -> Result<Option<i64>, Failure> {
    table.peek(item).map(decode).transpose()
}

/// `m6_stock *m6_stock_open(size_t capacity);`
///
/// The caller owns the stock and must pass it to [`m6_stock_close`] once.
#[unsafe(no_mangle)]
pub extern "C" fn m6_stock_open(capacity: usize) -> *mut Stock {
    let opened = catch_unwind(|| Table::new(capacity));
    match opened {
        Ok(Ok(table)) => Box::into_raw(Box::new(Stock { table })),
        Ok(Err(error)) => {
            set_last_error(&format!("cannot open a stock: {error}"));
            std::ptr::null_mut()
        }
        Err(payload) => {
            record_panic(payload);
            std::ptr::null_mut()
        }
    }
}

/// `void m6_stock_close(m6_stock *stock);`
///
/// # Safety
///
/// `stock` must be NULL or a pointer that [`m6_stock_open`] returned and that
/// has not been closed. It must not be used after this call, and neither may
/// the pointer from [`m6_stock_table`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m6_stock_close(stock: *mut Stock) {
    if stock.is_null() {
        return;
    }
    // SAFETY: a non-null `stock` came from `Box::into_raw` in `m6_stock_open`
    // and the caller promises that this is its only release.
    let stock = unsafe { Box::from_raw(stock) };
    // Dropping the Table calls kvtab_free. Nothing in that drop can panic,
    // but the guard costs nothing and keeps the rule simple: no exported
    // function lets a panic out.
    let _ = catch_unwind(AssertUnwindSafe(|| drop(stock)));
}

/// `int m6_stock_adjust(m6_stock *stock, const char *item, int64_t delta,
/// int64_t *quantity);`
///
/// # Safety
///
/// `stock` must be NULL or a live stock that no other call is using. `item`
/// must be NULL or a NUL-terminated string that is readable and unchanged
/// during the call. `quantity` must be NULL or valid for a write of one
/// `int64_t`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m6_stock_adjust(
    stock: *mut Stock,
    item: *const c_char,
    delta: i64,
    quantity: *mut i64,
) -> c_int {
    guard(|| {
        if stock.is_null() {
            return Err(Failure::null("stock"));
        }
        // SAFETY: `item` is NULL or a valid string for this call.
        let (item, name) = unsafe { item_name(item) }?;
        // SAFETY: `stock` is not NULL, so it is a live stock, and the caller
        // promises that no other call uses it now.
        let stock = unsafe { &mut *stock };
        let current = lookup(&stock.table, item)?.unwrap_or(0);
        let updated = current
            .checked_add(delta)
            .filter(|updated| *updated >= 0)
            .ok_or_else(|| {
                Failure::new(
                    M6_STOCK_ERR_RANGE,
                    format!("{name}: {current} + {delta} is outside 0..=INT64_MAX"),
                )
            })?;
        stock
            .table
            .put(item, &updated.to_le_bytes())
            .map_err(|error| match error {
                Error::Full => {
                    Failure::new(M6_STOCK_ERR_FULL, format!("{name}: the stock is full"))
                }
                other => Failure::internal(other),
            })?;
        if !quantity.is_null() {
            // SAFETY: a non-null `quantity` is valid for a write of one
            // `int64_t`. `write` does not read the old (maybe uninitialised)
            // value.
            unsafe { quantity.write(updated) };
        }
        Ok(())
    })
}

/// `int m6_stock_quantity(const m6_stock *stock, const char *item,
/// int64_t *quantity);`
///
/// # Safety
///
/// `stock` must be NULL or a live stock that no other call is changing.
/// `item` must be NULL or a NUL-terminated string that is readable and
/// unchanged during the call. `quantity` must be NULL or valid for a write of
/// one `int64_t`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m6_stock_quantity(
    stock: *const Stock,
    item: *const c_char,
    quantity: *mut i64,
) -> c_int {
    guard(|| {
        if stock.is_null() {
            return Err(Failure::null("stock"));
        }
        if quantity.is_null() {
            return Err(Failure::null("quantity"));
        }
        // SAFETY: `item` is NULL or a valid string for this call.
        let (item, name) = unsafe { item_name(item) }?;
        // SAFETY: `stock` is not NULL, so it is a live stock that nobody
        // changes during this call.
        let stock = unsafe { &*stock };
        let found = lookup(&stock.table, item)?
            .ok_or_else(|| Failure::new(M6_STOCK_ERR_UNKNOWN, format!("{name}: no such item")))?;
        // SAFETY: `quantity` is not NULL, so it is valid for one write.
        unsafe { quantity.write(found) };
        Ok(())
    })
}

/// `size_t m6_stock_len(const m6_stock *stock);`
///
/// # Safety
///
/// `stock` must be NULL or a live stock that no other call is changing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m6_stock_len(stock: *const Stock) -> usize {
    if stock.is_null() {
        return 0;
    }
    // SAFETY: `stock` is not NULL, so it is a live stock that nobody changes
    // during this call.
    let stock = unsafe { &*stock };
    // `Table::len` is one call of kvtab_len and cannot panic; the guard
    // keeps the rule without exception.
    catch_unwind(AssertUnwindSafe(|| stock.table.len())).unwrap_or(0)
}

/// `int m6_stock_report(const m6_stock *stock, char *buf, size_t buf_len,
/// size_t *needed);`
///
/// # Safety
///
/// `stock` must be NULL or a live stock that no other call is changing.
/// `buf` must be valid for writes of `buf_len` bytes, or `buf_len` must be 0.
/// `needed` must be NULL or valid for a write of one `size_t`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m6_stock_report(
    stock: *const Stock,
    buf: *mut c_char,
    buf_len: usize,
    needed: *mut usize,
) -> c_int {
    guard(|| {
        if stock.is_null() {
            return Err(Failure::null("stock"));
        }
        // SAFETY: `stock` is not NULL, so it is a live stock that nobody
        // changes during this call.
        let stock = unsafe { &*stock };
        let mut lines: Vec<(Vec<u8>, i64)> = Vec::new();
        let mut broken = None;
        stock
            .table
            .for_each(|key, value| match decode(value) {
                Ok(quantity) => {
                    lines.push((key.to_bytes().to_vec(), quantity));
                    ControlFlow::Continue(())
                }
                Err(failure) => {
                    broken = Some(failure);
                    ControlFlow::Break(())
                }
            })
            .map_err(Failure::internal)?;
        if let Some(failure) = broken {
            return Err(failure);
        }
        // Byte order of the names: `Vec<u8>` compares exactly like that.
        lines.sort();
        let mut report: Vec<u8> = Vec::new();
        for (name, quantity) in &lines {
            report.extend_from_slice(name);
            report.extend_from_slice(format!("={quantity}\n").as_bytes());
        }
        report.push(0);
        if !needed.is_null() {
            // SAFETY: a non-null `needed` is valid for one write.
            unsafe { needed.write(report.len()) };
        }
        if buf_len < report.len() {
            return Err(Failure::new(
                M6_STOCK_ERR_TOO_SMALL,
                format!(
                    "the report needs {} bytes, the buffer has {buf_len}",
                    report.len()
                ),
            ));
        }
        // The report has at least its NUL, so `buf_len >= 1` here and the
        // buffer must exist.
        if buf.is_null() {
            return Err(Failure::null("buf"));
        }
        // SAFETY: `buf` is valid for writes of `buf_len >= report.len()`
        // bytes, `report` is readable for its length, and a new `Vec` cannot
        // overlap the caller's buffer.
        unsafe {
            std::ptr::copy_nonoverlapping(report.as_ptr(), buf.cast::<u8>(), report.len());
        }
        Ok(())
    })
}

/// `int m6_stock_for_each(const m6_stock *stock, m6_stock_visit_fn visit,
/// void *userdata);`
///
/// # Safety
///
/// `stock` must be NULL or a live stock that no other call is changing.
/// `visit` must be NULL or a function that may be called with a
/// NUL-terminated string, a quantity and `userdata`, that does not change
/// the stock, and that does not unwind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m6_stock_for_each(
    stock: *const Stock,
    visit: StockVisitFn,
    userdata: *mut c_void,
) -> c_int {
    guard(|| {
        if stock.is_null() {
            return Err(Failure::null("stock"));
        }
        let visit = visit.ok_or_else(|| Failure::null("visit"))?;
        // SAFETY: `stock` is not NULL, so it is a live stock that nobody
        // changes during this call.
        let stock = unsafe { &*stock };
        // The call stack while `visit` runs:
        //   C program -> m6_stock_for_each (Rust) -> kvtab_for_each (C)
        //   -> the trampoline of Table::for_each (Rust) -> visit (C)
        let mut outcome = Ok(());
        stock
            .table
            .for_each(|key, value| {
                let quantity = match decode(value) {
                    Ok(quantity) => quantity,
                    Err(failure) => {
                        outcome = Err(failure);
                        return ControlFlow::Break(());
                    }
                };
                // SAFETY: the caller promises that `visit` accepts a
                // NUL-terminated string, a quantity and this `userdata`.
                // The key is valid until the closure returns.
                let stop = unsafe { visit(key.as_ptr(), quantity, userdata) };
                if stop != 0 {
                    outcome = Err(Failure::new(
                        M6_STOCK_ERR_STOPPED,
                        "the visitor stopped the walk",
                    ));
                    return ControlFlow::Break(());
                }
                ControlFlow::Continue(())
            })
            .map_err(Failure::internal)?;
        outcome
    })
}

/// `struct kvtab *m6_stock_table(const m6_stock *stock);`
///
/// # Safety
///
/// `stock` must be NULL or a live stock. The returned pointer is valid until
/// the stock is closed; the caller may only read through it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m6_stock_table(stock: *const Stock) -> *mut sys::kvtab {
    if stock.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: `stock` is not NULL, so it is a live stock. `as_raw` only
    // copies the handle and cannot panic.
    unsafe { &*stock }.table.as_raw()
}

/// `const char *m6_stock_last_error(void);`
#[unsafe(no_mangle)]
pub extern "C" fn m6_stock_last_error() -> *const c_char {
    // The pointer goes into the heap buffer of the stored CString. That
    // buffer lives until `set_last_error` replaces it, which happens only in
    // a failed call on this same thread: exactly what the header promises.
    LAST_ERROR
        .try_with(|slot| slot.borrow().as_ptr())
        .unwrap_or(c"".as_ptr())
}

/// `int m6_stock_debug_panic(void);`
#[unsafe(no_mangle)]
pub extern "C" fn m6_stock_debug_panic() -> c_int {
    guard(|| panic!("deliberate panic"))
}

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
        let p1: [u8; 3] = [2, 1, 0];
        assert_eq!(p1, [bytes[0], bytes[1], bytes[7]], "P1: little-endian");

        // P2. The range rule of m6_stock_adjust, as one expression. Write the
        // three results.
        let adjust = |current: i64, delta: i64| current.checked_add(delta).filter(|q| *q >= 0);
        let results = [adjust(3, -4), adjust(i64::MAX, 1), adjust(0, 0)];
        let p2: [Option<i64>; 3] = [None, None, Some(0)];
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
        let p3: [bool; 2] = [true, false];
        assert_eq!(p3, is_str, "P3: two payload types to handle");
    }
}
