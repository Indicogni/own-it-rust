# Mastery M6 hints

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Notes](NOTES.md)

Try the first hint before opening the second. Open *Check your explanation* only after writing your own answer to the lesson's EXPLAIN prompt.

## Lesson 01 · The C library again

<details><summary>Hint 1</summary>

Translate one C type at a time. `char **end` is a pointer to a `char *` that the function writes, so it is `*mut *mut c_char`. `div_t` needs `#[repr(C)]` and two `c_int` fields, `quot` first. For `errno` and `set_errno`, the supplied `errno_location()` gives you a `*mut c_int` that belongs to the calling thread: read through it, write through it. In `parse_long`, keep `let start = text.as_ptr();` and a `let mut end: *mut c_char = std::ptr::null_mut();`, and pass `&mut end`.

</details>
<details><summary>Hint 2</summary>

`parse_long`: check the radix first (`radix != 0 && !(2..=36).contains(&radix)`), call `set_errno(0)`, call `strtol`, and read `errno()` into a variable on the very next line. The number of consumed bytes is `end.cast_const().offset_from_unsigned(start)`; 0 means `NoDigits`. `divide`: return `None` for `denom == 0` and for `numer == c_int::MIN && denom == -1`, then read `.quot` and `.rem` of the result. `SortedInts::new`: `items.is_sorted().then_some(...)`. `find`: return `None` for an empty slice; pass `(&raw const needle).cast()` as the key, `self.items.as_ptr().cast()` as the base, the length, `size_of::<c_int>()` and `compare_c_int`; a non-null result becomes `found.cast_const().cast::<c_int>().offset_from_unsigned(base)`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`strtol` requires a NUL-terminated string, an end pointer that is NULL or writable, and a radix of 0 or 2 to 36. The first is guaranteed by the type `&CStr`, the second by passing `&mut end`, and the third is checked before the call. Overflow is not a precondition but a result, reported through `errno`, which the wrapper clears first because no successful C function sets it to 0. `div` requires a quotient that fits in an `int`; the wrapper checks the two cases where it does not. `bsearch` requires a valid array of `count` elements of `size` bytes, a comparison function that reads its arguments as that element type and does not unwind, and an array sorted in the order of that function. The first two are guaranteed by taking the pointer, the length and the element size from one slice and by using `compare_c_int`. The third cannot be checked cheaply on every call and must not be passed to a safe caller, so `SortedInts::new` checks it once and the private field carries it: `find` is sound for every `SortedInts` that can exist. Nothing is passed on to the caller, so every function here can be safe.

</details>

## Lesson 02 · Building C from Cargo

<details><summary>Hint 1</summary>

Three files. In `fletch-sys/Cargo.toml`, one line in `[package]`: `links = "fletch"`. In `build.rs`, `cc::Build::new()` with `.file(...)`, `.include(...)` and `.compile("fletch")`; `cc` prints the two link lines for you. A feature named `wide` appears to the build script as the environment variable `CARGO_FEATURE_WIDE`. In `src/lib.rs`, an `unsafe extern "C"` block with `pub fn` items; put `#[cfg(feature = "wide")]` on `fletch32`. Read `mastery/m6_ffi/build.rs` first: it does almost everything that you need, for another library.

</details>
<details><summary>Hint 2</summary>

Build the absolute include path from `CARGO_MANIFEST_DIR`: `PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("vendor/fletch/include")`. Print `cargo::rerun-if-changed=` for `build.rs`, `vendor/fletch/src/fletch.c` and `vendor/fletch/include/fletch.h`. Keep the builder in a variable (`let mut build = cc::Build::new();`) so that you can call `build.define("FLETCH_WIDE", None);` inside `if env::var_os("CARGO_FEATURE_WIDE").is_some()`. Finish with `println!("cargo::metadata=include={}", include.display());`. The declarations are `fletch16(data: *const u8, len: usize) -> u16`, `fletch32(data: *const u8, len: usize) -> u32` and `fletch_has_wide() -> c_int` (with `core::ffi::c_int`, because the crate is `no_std`). To see what your script printed, run `cargo build -p fletch-sys -vv` in `sys_lab/`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Cargo first builds the `cc` crate for the host, then compiles and runs the build script of `fletch-sys`. The script compiles `fletch.c` into `libfletch.a` in its `OUT_DIR` and prints its lines. Because of `links = "fletch"`, Cargo keeps the `metadata` line and runs the build script of `checker` only afterwards, with `DEP_FLETCH_INCLUDE` set. Then rustc compiles `fletch-sys` (the `rustc-link-lib` and `rustc-link-search` lines become `-l` and `-L` flags that are recorded in the library) and finally `checker`; the C archive is linked when the test executable is linked. The `rerun-if-changed` lines go to Cargo only: they decide whether the script runs again next time. If a second package in the graph also said `links = "fletch"`, Cargo would stop before compiling anything, because two packages would each bring their own copy of the same C symbols.

</details>

## Lesson 03 · Bindings by hand

<details><summary>Hint 1</summary>

Work through the header from top to bottom. The enumerators have explicit values; copy them. For the struct: `#[repr(C)]`, `size_t` is `usize`, `unsigned int` is `c_uint`, `unsigned char` is `c_uchar`, in the header's order. Each callback type is `Option<unsafe extern "C" fn(...) -> ...>`; a function that returns `void` has no `->`. In the block, `const kvtab *` is `*const kvtab` and `kvtab *` is `*mut kvtab`. If a clash error shows two signatures that look the same except for `sys::kvtab_stats` and `l03_bindings::kvtab_stats`, your struct is the difference: run the `b_` tests.

</details>
<details><summary>Hint 2</summary>

`declared` is twelve tuples such as `("kvtab_new", kvtab_new as *const ())`. `describe`: `unsafe { CStr::from_ptr(kvtab_strerror(status)) }.to_string_lossy().into_owned()`; the SAFETY comment is that the function returns a static, NUL-terminated string for every `int`. `measure`: return `None` if `kvtab_new` gives NULL; loop over the pairs with `kvtab_put(table, key.as_ptr(), value.as_ptr(), value.len())` and remember whether a status was not `KVTAB_OK` (do not `return` inside the loop, or the table leaks); then `let mut stats = MaybeUninit::<kvtab_stats>::uninit();`, `kvtab_get_stats(table, stats.as_mut_ptr())`, `kvtab_free(table)`, and only if everything was OK `Some(unsafe { stats.assume_init() })`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A new field in the middle of the C struct changes its size and moves every later field. The layout tests would notice at once: `size_of` and at least one `offset_of!` no longer agree with what `layout_probe.c` reports, because the probe is compiled from the new header. bindgen's generated layout test would notice for the same reason, if the bindings were generated again. The `clashing_extern_declarations` check would notice only if the reference bindings had been updated; it compares two Rust translations, not Rust with C. Miri would notice nothing: the model is written against the old struct too. Without any check, `kvtab_get_stats` would write more bytes than the Rust struct has, which overwrites whatever is next on the stack, and `measure` would return a struct whose later fields hold the values of other fields. That is the general lesson: only a check that involves the C compiler, or a tool that reads the header, can find a binding that is out of date.

</details>

## Lesson 04 · A safe wrapper

<details><summary>Hint 1</summary>

`check` is one `match` with a catch-all arm; `status` is the same `match` backwards. For `Display`, write a small closure or function that turns `sys::kvtab_strerror(code)` into text with `CStr::from_ptr(...).to_string_lossy()`, and use it for the four variants that have a code. Store the handle as `NonNull<sys::kvtab>`: that makes the type one pointer wide and gives `Option<Table>` the NULL pattern. `Table::new` turns a NULL result into `Error::NoMemory` with `NonNull::new(raw).ok_or(...)`.

</details>
<details><summary>Hint 2</summary>

Add `impl Drop for Table` that calls `sys::kvtab_free(self.raw.as_ptr())`. `into_raw` must not run that `Drop`: `ManuallyDrop::new(self).raw.as_ptr()`. `from_raw` is `Table { raw: unsafe { NonNull::new_unchecked(raw) } }`. `put` and `remove` end with `Error::check(status)`. `stats` uses `MaybeUninit::<sys::kvtab_stats>::uninit()`, checks the status with a real check (`expect`, not `debug_assert`), then `assume_init`, and converts `full != 0`. For threads: `NonNull` makes the type neither `Send` nor `Sync`, and from that moment the test module does not compile (E0277 in the test that moves a table to another thread); add `unsafe impl Send for Table {}` and quote the "Threads" paragraph of kvtab.h in the SAFETY comment. Do not add `Sync`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The invariant: `raw` points to a live table that `kvtab_new` created, and this `Table` is its only owner. `new` and `from_raw` establish it (the second by making it the caller's duty). `len`, `stats`, `put` and `remove` rely on it for "the handle is live". `Drop` relies on it and ends it; `into_raw` ends it without freeing, which is why it must prevent `Drop`. `as_raw` cannot break it by itself, but hands the caller the means to, which its documentation says. No safe method can break the invariant, because the field is private and the type is not `Clone`. `put` takes `&mut self` although the compiler would accept `&self`, for two reasons that both come from the header: `kvtab_put` changes the table, and it frees the old value, so it ends the validity of pointers that `kvtab_peek` returned. With `&mut self`, a borrowed view that is tied to `&self` (lesson 06) cannot exist during a `put`. The same choice also keeps an observer (lesson 05) from being called while another call is in progress.

</details>

## Lesson 05 · Callbacks

<details><summary>Hint 1</summary>

For `for_each`, write `unsafe extern "C" fn visit_trampoline<F>(key, value, value_len, userdata) -> c_int where F: FnMut(&CStr, &[u8]) -> ControlFlow<()>`. Inside, cast `userdata` back, build the `&CStr` and the `&[u8]`, call the closure, and return 0 for `Continue` and 1 for `Break`. In `for_each`, pass `Some(visit_trampoline::<F>)` and a pointer to your local closure (`(&raw mut f).cast()`). `KVTAB_ERR_STOPPED` means that the closure asked to stop, so it is `Ok(())`. For the observer, `Box::into_raw(Box::new(f)).cast()` is the userdata, and you need a second generic function, `unsafe extern "C" fn drop_closure<F>(userdata: *mut c_void)`, that does `Box::from_raw`.

</details>
<details><summary>Hint 2</summary>

The panic: make `userdata` point to a struct with two fields, the closure and `panic: Option<Box<dyn Any + Send>>`. In the trampoline, `match catch_unwind(AssertUnwindSafe(|| (walk.visit)(key, value)))`: on `Err(payload)` store the payload and return 1. After `kvtab_for_each` has returned, `if let Some(payload) = walk.panic { resume_unwind(payload) }`. Build the value slice with `if value_len == 0 { &[] } else { from_raw_parts(value, value_len) }`. `observe` passes `Some(observe_trampoline::<F>)`, the boxed closure and `Some(drop_closure::<F>)` to `kvtab_set_observer`; the observer trampoline maps `KVTAB_EVENT_PUT` and `KVTAB_EVENT_REMOVE` to `Event` and returns for anything else. `clear_observer` calls `kvtab_set_observer(raw, None, null_mut(), None)`; the library drops the old closure through the drop function that came with it.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

In `for_each`, `userdata` points to a local variable of `for_each` that holds the closure and the slot for a panic. `for_each` owns it, it lives until `for_each` returns, which is after `kvtab_for_each` has returned, and `for_each` does not touch it while C runs; the library calls the trampoline from this thread, one call at a time, so the `&mut` in the trampoline is the only reference. In `observe`, `userdata` is a `Box<F>` turned into a raw pointer. The C table owns it until it calls `drop_closure::<F>`, once, when the observer is replaced or the table is freed. The library calls the observer only from inside `kvtab_put` and `kvtab_remove`, which the wrapper reaches only through `&mut self`, so no two calls overlap and no replacement can happen during a call. When the `for_each` closure panics, `catch_unwind` in the trampoline catches it, the payload is stored, the trampoline returns 1, `kvtab_for_each` stops and returns normally, and `for_each` continues the same panic with `resume_unwind`; the C frame was never unwound. When an observer panics, the panic reaches the end of an `extern "C"` function and the process aborts. That is this wrapper's documented policy, because the C function that called the observer has no way to report a failure.

</details>

## Lesson 06 · Strings and buffers

<details><summary>Hint 1</summary>

`version`: `unsafe { CStr::from_ptr(sys::kvtab_version()) }` with the type `&'static CStr`, then `to_str().unwrap_or("unknown")`. `get_into`: `let mut len = buf.len();`, call `kvtab_get` with `buf.as_mut_ptr()` and `&mut len`; `KVTAB_ERR_TOO_SMALL` becomes `BufferTooSmall { needed: len }`, everything else goes through `Error::check`. `peek`: call `kvtab_peek` with a `&mut len`; NULL is `None`; otherwise `slice::from_raw_parts(value, len)`. Leave the lifetimes of `peek` elided: then the slice borrows `self`.

</details>
<details><summary>Hint 2</summary>

`get`: first call with `std::ptr::null_mut()` and `len = 0`. If the status is not `KVTAB_ERR_TOO_SMALL`, return `Error::check(status).map(|()| Vec::new())` (that covers the empty value and the missing key without an allocation). Otherwise `Vec::with_capacity(len)`, a second call with `value.as_mut_ptr()` and `let mut written = value.capacity();`, `Error::check`, an `assert!(written <= value.capacity())`, and `unsafe { value.set_len(written) }`. `put_str`: `CString::new(key).map_err(|_| Error::InvalidKey)?`, then `self.put(&key, value.as_bytes())`. `get_string`: the same key conversion, `self.peek(&key).ok_or(Error::NotFound)?`, then `String::from_utf8_lossy(bytes).into_owned()`. `to_wide`: refuse `text.contains('\0')`, then `text.encode_utf16().chain(std::iter::once(0)).collect()`. `from_wide`: count units until the 0 with `*text.add(len)`, then `String::from_utf16_lossy(slice::from_raw_parts(text, len))`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Keys go from Rust to C as `key.as_ptr()`: Rust allocated them, Rust frees them, and C may use the pointer during the call only (the library copies the key). The same holds for the value in `put`. The version string comes from C: static storage, nobody frees it, valid forever, so `&'static CStr` is the true type. In `get_into` and `get`, Rust owns the buffer before and after; C writes into it during the call and keeps nothing. In `peek`, the bytes were allocated by the library and will be freed by the library; Rust may read them until the next `kvtab_put`, `kvtab_remove` or `kvtab_free` on the table. The slice cannot outlive the next `put`, because `peek` takes `&self` and returns a slice with the same lifetime, and `put` takes `&mut self`: the borrow checker refuses a `put` (or a `remove`, a drop, an `into_raw`) while the slice is alive. The line that enforces it is the signature `fn peek(&self, key: &CStr) -> Option<&[u8]>`, together with the `&mut self` of lesson 04. For `to_wide` and `from_wide`, Rust allocates and frees the vector; the foreign side may read it during the call, and `from_wide` only reads the caller's memory up to the 0 unit.

</details>

## Code review 01 · A teammate's wrapper

<details><summary>Hint 1</summary>

Go through the methods with the header open, and ask the three questions of each call: is the status checked, can anything here panic, and what is the worst input? `load`: what does `kvtab_new` return for no pairs? What happens to a name with a NUL byte? What does `pairs.clone()` cost? `get`: which statuses can `kvtab_get` return, and what is in `len` after each? `names`: who is the caller of `collect`, and what happens to a panic inside it? Then look for what is *not* there: who frees the table?

</details>
<details><summary>Hint 2</summary>

Store the handle as `NonNull<sys::kvtab>` and add `Drop`. In `load`, create the table with `pairs.len().max(1)`, build the `Settings` before the loop so that `Drop` frees the table on an early return, borrow the pairs (`&[(String, String)]`, `for (name, value) in pairs`), map `CString::new(name.as_str())` errors to `InvalidName`, and return `Library(status)` for a failed put. In `get`, `CString::new(name).ok()?`, then `kvtab_peek`: NULL is `None`, otherwise decode the slice with `from_utf8_lossy` (no fixed buffer, and no second call). In `collect`, use `to_string_lossy()` instead of `to_str().unwrap()`. Delete `unsafe impl Sync`. Write a SAFETY comment for each `unsafe` block and for `unsafe impl Send`.

</details>
<details><summary>Check your review (after writing your own)</summary>

Bugs. (1) No `Drop`: every `Settings` leaks its C table. (2) `get` ignores the status of `kvtab_get`: for a missing name it returns `Some` of 32 zero bytes, and for a value longer than 32 bytes the library sets `len` to the needed size, so `&buf[..len]` panics. (3) `load(&vec![])` passes capacity 0, gets NULL, and stores it; every later call then fails quietly inside C, which `get` turns into a wrong answer. (4) `CString::new(name).unwrap()` panics on a name with a NUL byte, in `load` and in `get`, although `load` has an error type for exactly this. (5) `to_str().unwrap()` in `collect` panics for a key that is not UTF-8, and `collect` is an `extern "C"` function called by C: the process aborts. (6) `unsafe impl Sync` is false: the header forbids two threads inside calls on one table at the same time, whatever the functions are, and `get`, `len` and `names` take `&self`.

Smells. `load` ignores the status of `kvtab_put`. `&Vec<(String, String)>` should be a slice, and `pairs.clone()` copies every string for nothing. No `unsafe` block has a SAFETY comment, and the comment on the impls ("it is just a pointer") is an argument for the opposite conclusion: a raw pointer is exactly the case where the compiler cannot know. The `as` casts of pointers hide what is being converted; `.cast()` and `&raw mut` say it. The fixed buffer is a design decision that should have been a question: "how long can a value be?" has the answer "any length" in the header.

</details>

## Lesson 07 · Exporting a Rust library to C

<details><summary>Hint 1</summary>

Start with the header: an opaque `typedef struct m6_tally m6_tally;`, three `#define`s, seven prototypes with `uint32_t` and `uint64_t` from `<stdint.h>`. On the Rust side, a struct with a `BTreeMap<String, u64>`, and for each prototype a `#[unsafe(no_mangle)] pub extern "C" fn` (or `pub unsafe extern "C" fn` when it trusts a pointer). `m6_tally_new` is `Box::into_raw(Box::new(...))`; `m6_tally_free` checks for NULL and does `drop(unsafe { Box::from_raw(tally) })`. Read `ctests/l07_tally_test.c`: each `CHECK` is one sentence of the contract.

</details>
<details><summary>Hint 2</summary>

`m6_tally_add`: return `M6_TALLY_ERR_NULL` if either pointer is NULL; `let Ok(text) = unsafe { CStr::from_ptr(text) }.to_str() else { return M6_TALLY_ERR_UTF8 };`; then `for word in text.split_ascii_whitespace()` with `entry(word.to_owned()).or_insert(0)` and `saturating_add(1)` (a plain `+= 1` can panic in a debug build). `m6_tally_count`: 0 for NULL or bad UTF-8, else `counts.get(word).copied().unwrap_or(0)`. `m6_tally_top`: the map iterates in byte order and `max_by_key` returns the last maximum, so use `counts.iter().rev().max_by_key(|(_, count)| **count)`; turn the word into `CString::new(word.as_str())` and return `into_raw()`, or NULL. `m6_tally_string_free`: NULL check, then `drop(unsafe { CString::from_raw(text) })`. The C program writes into the string before it gives it back, which is allowed as long as the length stays the same; say so in the header, because `CString::from_raw` searches for the NUL to learn the size of the allocation. If the linker reports an undefined symbol, compare the spelling with the header and look for a missing `#[unsafe(no_mangle)]` (`pub` plays no part in the export).

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The string of `m6_tally_top` was allocated by Rust's global allocator, as a `Box<[u8]>` inside a `CString`. C's `free` belongs to the C library's allocator. The two may be different allocators, and even when they are the same one, `CString::from_raw` must know the length of the allocation to release it, which it finds by scanning for the NUL (so the C caller may change the bytes but must never change the length). Passing the pointer to `free` is undefined behavior: it may corrupt the heap, crash later in an unrelated place, or appear to work. So the library exports `m6_tally_string_free`, which gives the pointer back to the allocator that made it. Two designs avoid the second function. The caller can provide the buffer and its size, and the function returns the size that it needs (lesson 06's two-call protocol; the capstone's report does this). Or the library keeps the string and returns a borrowed pointer that is valid until a documented later event, such as the next call on the same tally. A third option, using C's `malloc` from Rust so that C's `free` is correct, works too, but needs a declaration of `malloc` and care that both sides link the same C library.

</details>

## Lesson 08 · Capstone

<details><summary>Hint 1</summary>

Do not start with the exported functions. Start with three helpers: a `Failure { status, message }` type; `fn guard(body: impl FnOnce() -> Result<(), Failure>) -> c_int` that runs the body inside `catch_unwind(AssertUnwindSafe(body))`, stores the message of a failure or of a panic in a `thread_local!` `RefCell<CString>`, and returns the status; and a function that turns a `*const c_char` item name into a checked `&CStr` and `&str`. Then every exported function is `guard(|| { ... Ok(()) })` with `?` inside. The handle is `Box<Stock>` where `Stock` contains your `Table`. A quantity is `value.to_le_bytes()` going in and `i64::from_le_bytes(<[u8; 8]>::try_from(bytes)?)` coming out.

</details>
<details><summary>Hint 2</summary>

`adjust`: read the current quantity with `peek` (missing means 0), compute `current.checked_add(delta).filter(|q| *q >= 0)` and return `M6_STOCK_ERR_RANGE` for `None` *before* any `put`, then `put` and map `Error::Full` to `M6_STOCK_ERR_FULL`; write `*quantity` last, with `quantity.write(updated)`, and only if it is not NULL. `report`: collect `(name bytes, quantity)` with `for_each`, sort, build a `Vec<u8>` ending in 0, write `*needed` if it is not NULL, return `TOO_SMALL` before touching `buf`, then `copy_nonoverlapping`. `for_each`: inside your `Table::for_each` closure call the C function pointer with `key.as_ptr()`, the decoded quantity and `userdata`; remember in a local variable whether the visitor stopped, and return `Break`. `last_error`: `LAST_ERROR.try_with(|slot| slot.borrow().as_ptr()).unwrap_or(c"".as_ptr())`. The text must be a `thread_local!`, not one global `static`: with a global, a failure on another thread would free the string that this thread is still reading (the Miri test checks this with two threads). The panic message: the payload is a `&'static str` for `panic!("literal")` and a `String` when the message is formatted; try both downcasts. `m6_stock_debug_panic` is `guard(|| panic!("deliberate panic"))`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A good review names, for each function, what the header demands and what the code checks. Every function checks for NULL where a pointer is required, checks that item names are UTF-8, and checks the range of quantities with `checked_add`. No function can check that a non-null `stock` is live and not in use on another thread, that `buf` really has `buf_len` writable bytes, that `quantity` and `needed` point to writable memory of the right type, or that `visit` is a function with the declared signature that does not change the stock; these are in the header as comments and in `# Safety` sections, and breaking one is undefined behavior that no test can turn into an error code. The change that breaks C programs silently is any change to a signature, the order or meaning of parameters, or the value of a status code: an old program still compiles against its old header, or is not recompiled at all, and passes or reads the wrong thing. Adding a function is safe; changing the fields of `Stock` is safe, because the type is opaque. `m6_stock_table` gives that freedom away in one respect: by handing out the kvtab handle and documenting the storage format (one entry per item, 8 little-endian bytes), it makes the table and the encoding part of the interface. Changing the storage later, for example to a Rust `HashMap` or to a variable-length encoding, would break every caller that reads the table. An escape hatch is convenient, and it is also a promise.

</details>

## Drills

Run `cargo drill m6`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · A closure where C wants a function pointer</summary>E0308. A closure that captures `step` is code plus data, and a function pointer is one address. Only closures that capture nothing can become `fn` pointers, and then only pointers with the Rust ABI, not `extern "C"` ones. Write a named `extern "C" fn`. When the callback needs data, the C interface must have a `void *userdata` parameter (lesson 05).</details>
<details><summary>d02 · Matching on a status code</summary>E0004. The matched value is a `c_int`, and the three constants cover three of its four billion values. Add `_ => "unknown"`. The compiler is pointing at a real case: a newer library may return a new code. This is also why status codes are integer constants and not a Rust `enum`.</details>
<details><summary>d03 · An out-parameter</summary>E0381: `stats` is used (borrowed with `&mut`) before it is initialised. A reference must point to an initialised value; the compiler cannot know that the callee only writes. Use `MaybeUninit::<Stats>::uninit()`, pass `as_mut_ptr()`, and call `assume_init()` after the call.</details>
<details><summary>d04 · A handle on another thread</summary>E0277: `*mut u32` is not `Send`, so `Handle` is not, so the closure is not. The compiler knows nothing about what a raw pointer points to. The author of the type does: a `Handle` is the only owner of its allocation, like a `Box`. Say so with `unsafe impl Send for Handle {}` and a SAFETY comment. With a real C library the evidence must come from its documentation.</details>
<details><summary>d05 · A borrowed view and a later write</summary>E0502. `peek(&self)` returns a slice that borrows the table, and `put(&mut self)` needs the table exclusively while the slice is still used. That is the signature doing its job: it encodes "valid until the next put". Copy the bytes (`.to_vec()`) if you need them after the write.</details>
<details><summary>d06 · Giving a handle away</summary>Panics: the second assertion fails. `into_raw(self)` takes ownership of the handle, returns the id, and then `self` goes out of scope and `Drop` releases the resource that the caller now thinks it owns. Stop the destructor with `ManuallyDrop::new(self)` or `std::mem::forget(self)`. With a real library this bug is a use after free.</details>
<details><summary>d07 · Translating a struct</summary>Panics: the size is 24, not 16. `repr(C)` keeps the fields in the order written: `tag` at 0, seven bytes of padding, `id` at 8, `flag` at 16, seven more bytes of padding. The header's order is `id, tag, flag`, which gives 16. The size check failing is the lucky case; the real damage is that every field after the first is at the wrong offset.</details>
<details><summary>d08 · Catching a panic from a C-ABI function</summary>Panics, and more precisely aborts: "panic in a function that cannot unwind". A function with the ABI `"C"` never unwinds; when a panic tries to leave it, the process is aborted before `catch_unwind` in the caller sees anything. Here the caller is Rust, so `extern "C-unwind"` is the smallest fix. When the caller is C, catch the panic *inside* the function instead.</details>
<details><summary>d09 · A callback that may be absent</summary>Compiles and runs. `Option<extern "C" fn()>` has the size of one pointer, and `None` is the all-zero pattern. The language guarantees this for `Option` of a function pointer, a reference, a `Box` and a `NonNull`, which is why a nullable C callback is declared this way.</details>
<details><summary>d10 · Two modules, one symbol name</summary>error (no error code): "symbol `init` is already defined". `no_mangle` exports each function under its bare name. Rust tells the two apart by their module paths; the linker has one flat namespace. Give each a prefixed symbol with `#[unsafe(export_name = "parser_init")]`, or rename the functions.</details>

## Mastery check: model answers

<details><summary>1. What an extern block promises, and how to check a binding</summary>

It promises the compiler that a symbol with this name exists at link time and that its real definition has exactly this signature: the same number and order of parameters, types with the same size, alignment and way of being passed, the same return type, the C calling convention. The compiler cannot check any of it, and a wrong promise is undefined behavior at the first call. Three independent checks: compile a small C file that reports `sizeof`, `offsetof` and constant values, and compare in a test; generate the bindings with bindgen, which reads the header with a C parser, and compare or use its layout tests; run under Miri against a definition (a model, or the exported Rust functions), which checks each call's types. A fourth, weaker one: a second hand-written declaration and the `clashing_extern_declarations` lint, which compares two translations with each other.

</details>
<details><summary>2. From cargo build to rustc, links, metadata, and inputs</summary>

Cargo compiles the build dependencies for the host, compiles `build.rs`, and runs it with `OUT_DIR`, `TARGET`, the profile and the features in its environment. The script (through `cc`) finds the C compiler, compiles the sources into objects, archives them into `OUT_DIR`, and prints `rustc-link-lib` and `rustc-link-search`, which Cargo turns into `-l` and `-L` flags for rustc. `links = "name"` declares that the package links that native library; Cargo allows one such package per graph, so the library is never linked twice. It also makes `cargo::metadata=KEY=VALUE` lines available to the build scripts of direct dependents as `DEP_NAME_KEY`. A script must print `rerun-if-changed` for each C input because `cc` prints `rerun-if-env-changed` lines, and after the first `rerun-if` line Cargo no longer reruns the script for changes in the package directory: an edited C file would be ignored.

</details>
<details><summary>3. Enums, function pointers, opaque structs</summary>

A Rust `enum` may hold only its listed values, and any other value is undefined behavior as soon as it exists; a C function may return a value that the bindings do not know, and the integer type of a C enum is not even the same on every compiler. Integer constants have neither problem. A C function pointer may be NULL and a Rust `fn` pointer may not; `Option<extern "C" fn>` has the same size and ABI, with `None` as NULL. An opaque struct is `#[repr(C)] struct T { _data: [u8; 0], _marker: PhantomData<(*mut u8, PhantomPinned)> }`. The private fields prevent construction outside the module; the zero-sized array gives it no size that anyone could rely on; `*mut u8` in the marker removes `Send` and `Sync`, so that the binding makes no claim about threads; `PhantomPinned` removes `Unpin`, because the C object must not be moved by Rust.

</details>
<details><summary>4. The invariant of an owning handle, and Send and Sync</summary>

"The pointer is a live object from the library's constructor, and this value is its only owner." Constructors establish it; `Drop` relies on it to call the library's free function exactly once; `into_raw` transfers the ownership out and must therefore prevent `Drop`; `from_raw` is `unsafe` because it makes the invariant the caller's duty. The type must not be `Clone` unless the library can really copy the object. For `Send` you need the library's documentation to say that an object is not tied to a thread and uses no unsynchronised global state, and every Rust value that the object owns (such as a boxed closure) must be `Send` too. For `Sync` you need more: the documentation must allow the functions behind your `&self` methods to run at the same time on one object. Reading today's source code is not that evidence.

</details>
<details><summary>5. Closures as callbacks</summary>

The closure's data goes through `void *userdata`, and its code through a generic `extern "C"` trampoline that casts `userdata` back to the closure's type and calls it. For a callback that runs during one call, the closure stays on the caller's stack, `userdata` points to it, nothing is freed, and no `'static` bound is needed. For a callback that C keeps, the closure is boxed, `userdata` is `Box::into_raw`, and a drop function that the library calls exactly once does `Box::from_raw`; the closure must be `'static`, and `Send` if its owner can move between threads. A panic in a callback must not unwind into C. Either catch it in the trampoline, make the library stop, and `resume_unwind` after the C function has returned; or let the `extern "C"` function abort the process and document that; or use `"C-unwind"`, which is correct only if the foreign code was built to be unwound through.

</details>
<details><summary>6. Pointers across the boundary, and peek</summary>

Who allocated the memory, who frees it and with which function, and until when the other side may use the pointer. `peek(&self) -> Option<&[u8]>` returns a slice that borrows the table. The library ends the pointer's validity at the next put, remove or free; the wrapper reaches those only through `&mut self` methods or by dropping or consuming the table, and the borrow checker refuses all of them while the slice is alive. It would be unsound if any method that changes the table took `&self`, if the type were `Sync` or `Clone`, if the returned lifetime were not tied to `self` (for example `'static`), or if a safe method handed the raw table to code that changes it.

</details>
<details><summary>7. Exporting: what is needed, who checks, what may change</summary>

A function with the `extern "C"` ABI, `#[unsafe(no_mangle)]` (or `export_name`) so that the symbol has a predictable name and is exported, FFI-safe types in the signature, and a crate type that a C linker understands. `pub` is not needed for the export; it only decides what Rust code and the documentation see. `lib` produces an `rlib` for Rust; `staticlib` an archive with your code and the Rust standard library, which needs the system libraries that `--print native-static-libs` lists; `cdylib` a shared library that exports only the C symbols. Nobody checks the header against the Rust code unless you arrange it: generate it with cbindgen, or test both against a third description, as this module does (the C compiler compares the header with the test program's function pointer types, and Miri compares the Rust definitions with Rust declarations of the same contract). Silent breakage: changing a signature, a struct that C can see into, or the value of a constant. Safe: adding functions, and changing anything behind an opaque handle.

</details>
<details><summary>8. The guard</summary>

It runs the body of the exported function inside `catch_unwind`, turns a panic into an error value (a status code, or NULL), and stores a description where the caller can get it, typically in a thread-local that an exported "last error" function reads. What can still go wrong: the code that handles the caught panic can itself panic (formatting, a thread-local that is already destroyed, the payload's destructor), and that panic is outside the guard, so keep that code trivial, use `try_with`, and drop the payload inside a second `catch_unwind`; and the object that the body was changing may be left half changed, so either make each change a single step or mark the object unusable. The caller learns the status from the return value and the text from the "last error" function, whose pointer is valid only until the next failed call on the same thread.

</details>
