# Chapter 24 hints — Calling C

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Try the first hint before opening the second. The implementation constraints in each task are part of the exercise. The sections follow the study order: lesson 08 comes before the capstone (l07).

## l01 · C representations

<details><summary>Hint 1</summary>

Use std::ffi aliases and repr(C) to express the platform C ABI. They do not serialize pointers or Rust-owned collections.

</details>

<details><summary>Hint 2</summary>

Measure layout with size_of, align_of, and offset_of rather than hardcoding this machine’s sizes.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

`c_char`, `c_int`, `c_uint`, and the other `std::ffi` aliases name whatever the target's C compiler uses. `c_int` is 4 bytes on the usual desktop targets. `c_long` is 8 bytes on 64-bit Unix but 4 on Windows, and `c_char` is signed on x86-64 but unsigned on ARM Linux, which is why portable code uses the alias instead of a fixed Rust type. `#[repr(C)]` lays fields out in declaration order with C's padding rules: `Reading` puts 3 padding bytes after `kind` so `count` is 4-aligned. repr(C) does not make a field's *type* C-compatible: a `String` field is still a Rust String. A raw byte dump of the struct is not a portable wire format: the padding is uninitialized, and byte order, sizes, and alignment differ between targets. Serialize each field explicitly instead.

</details>

## l02 · C strings and ownership

<details><summary>Hint 1</summary>

CStr validates termination, not UTF-8. Owning the text requires an actual copy.

</details>

<details><summary>Hint 2</summary>

Use CString::new, CStr::to_str, and CStr::from_bytes_with_nul. No arbitrary-pointer scan is needed for the bounded-buffer task.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

`CString` owns a NUL-terminated buffer with no interior NUL. It is what you build and keep alive while C reads it, and `CString::new` rejects an interior NUL because C would stop reading there. `CStr` is the borrowed view, like `&str` for C strings. `as_ptr()` gives a raw pointer that is valid only while the owner lives and is not modified; the type system no longer tracks it, so a pointer taken from a temporary CString dangles at the end of the statement. Neither type promises UTF-8: `to_str()` validates and can fail, and copying into a Rust `String` (`to_owned`) is what lets the text outlive the C buffer. `from_bytes_with_nul` requires exactly one NUL, at the end; `from_bytes_until_nul` stops at the first one.

</details>

## l03 · ABI calls and out-parameters

<details><summary>Hint 1</summary>

MaybeUninit lets you allocate output storage without inventing an initialized value. Its contents become valid only after the documented success status.

</details>

<details><summary>Hint 2</summary>

Call with out.as_mut_ptr, inspect status, then use assume_init only in the success branch. Preserve the extern C function-pointer type.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

A C function that reports status and writes results through an out-pointer initializes the out slot only on success. So the wrapper passes `MaybeUninit<c_int>` storage and calls `assume_init` only when the status says success; on failure the slot may still be garbage. Reading it would be undefined behavior, which is why the prediction used an initialized slot. `extern "C"` only selects the calling convention. It says nothing about whether the call is safe: the declaration may be wrong, and the callee may have preconditions. A function pointer's type includes its ABI, so `extern "C" fn(c_int, c_int) -> c_int` and `fn(i32, i32) -> i32` are different types. Mixing them up would make caller and callee disagree about where arguments live.

</details>

## l04 · Callbacks and userdata

<details><summary>Hint 1</summary>

The pointer is borrowed, not transferred. This mock API promises callbacks finish before foreign_map returns.

</details>

<details><summary>Hint 2</summary>

Create a Vec of initialized output slots, pass as_ptr/as_mut_ptr and a c_void cast of a local factor, then return the Vec after the call. Call foreign_map once for the whole slice: a test counts its callbacks, so a Rust loop that multiplies the values itself fails.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

The callback receives the userdata as `*mut c_void` and casts it back. The pointer is only valid while the referent lives, so the factor, the input, and the output must all outlive the *synchronous* call, and the callee must not keep the pointer. A library that stores the pointer for later calls needs the userdata kept alive (for example boxed and leaked) until it is unregistered. `Option<extern "C" fn ...>` has the same layout as a nullable C function pointer, so `None` is NULL. A Rust panic must not unwind out of an `extern "C"` function: since Rust 1.81 it aborts the process. That is why `scale` uses `saturating_mul`, which cannot panic, and why arbitrary closures need `catch_unwind` in a trampoline.

</details>

## l05 · Opaque handle ownership

<details><summary>Hint 1</summary>

The owner has one non-null pointer and no Clone. Drop is supplied to preserve safety even while exercise functions are unfinished.

</details>

<details><summary>Hint 2</summary>

Create CString before raw_open. Borrow the Resource only while &self exists, then allocate owned output with to_owned.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

`Owner::open` validates the name first, so a NUL error never creates a resource. It then takes the unique ownership that `raw_open` hands out and stores it as `NonNull`, which records "never null" in the type (and gives `Option<Owner>` a niche). `Drop` calls the matching `raw_close` exactly once. The owner cannot be cloned, so no second owner can free the handle again. Every allocation must go back to its own deallocator. The mock pairs `Box::into_raw` with `Box::from_raw`. A real C library allocates with its own allocator, so it must provide its own free function, and calling `Box::from_raw` or `free` on the wrong pointer is undefined behavior. `name_owned` copies the text so the String outlives the handle.

</details>

## l06 · Owner-bound foreign borrows

<details><summary>Hint 1</summary>

The lifetime must appear in the view’s type even though the raw pointer does not carry a Rust lifetime itself.

</details>

<details><summary>Hint 2</summary>

Use PhantomData<&Repository>. Tie construction to &self, then justify from_ptr from the immutable CString that remains live.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

A raw `*const c_char` carries no lifetime, so it cannot say how long the name stays valid. `NameView<'repo>` adds `PhantomData<&'repo Repository>`. To the borrow checker the view borrows the repository, so the repository cannot be dropped or mutated while a view exists, and the view costs nothing at run time. `as_cstr` returns `&'repo CStr`, tied to the owner rather than to the view, which is sound because the CString's heap buffer does not move and the private field is not modified during the borrow. An owned copy (`to_owned`) has no such tie and survives the repository. Real libraries have the same shape. In SQLite, for example, a prepared statement is valid only while its database connection is open, so a safe wrapper ties `Statement<'conn>` to `&'conn Connection`.

</details>

## l08 · Calling the real C library

Do this lesson before the capstone.

<details><summary>Hint 1</summary>

A: translate each prototype type by type, in the order in which it is written. A compile error "redeclared with a different signature" shows the expected signature next to yours. B: what does a `&CStr` already guarantee about its buffer? That guarantee is your SAFETY comment. C: decide in Rust, before the call, which input C cannot handle. D: read both integers through the pointers first, then compare them without arithmetic. E: `qsort` needs four things, and a slice gives you three of them: its pointer, its length, and the size of its element type.

</details>

<details><summary>Hint 2</summary>

A: `fn strlen(s: *const c_char) -> usize;`, `fn abs(n: c_int) -> c_int;`, and `qsort` with `base: *mut c_void`, two `usize` parameters, and `compar: Comparator`. B: `unsafe { strlen(text.as_ptr()) }`. C: `if n == c_int::MIN { return None; }`, then `Some(unsafe { abs(n) })`. D: `let (a, b) = unsafe { (*a.cast::<c_int>(), *b.cast::<c_int>()) };` then `a.cmp(&b) as c_int`, or `(a > b) as c_int - (a < b) as c_int`. E: return early for `items.len() < 2`, then `qsort(items.as_mut_ptr().cast(), items.len(), size_of::<T>(), compare)`.

If the whole test run stops with "panic in a function that cannot unwind" and "aborting", your comparator panicked. Scroll up to the first panic message: it names the line. The usual cause is `a - b`, which overflows for `c_int::MIN` and `c_int::MAX`.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

An `unsafe extern "C"` block is a list of promises: each declaration says that a function with this name exists and takes exactly these types under the C calling rules. Nobody checks the promise. The compiler cannot read C headers, and the linker matches names only, so a wrong declaration still builds and then misbehaves. (The exercise adds a check that real projects do not have: a second declaration, and the `clashing_extern_declarations` lint as an error.) Each function in the block is unsafe to call because it has preconditions. `c_strlen` meets `strlen`'s precondition through its parameter type: a `&CStr` is always a live buffer that ends with a NUL. `checked_c_abs` rejects `c_int::MIN`, the one input for which C's `abs` is undefined. `c_sort` passes the count and the element size of the same slice, so `qsort` stays inside the buffer, and it leaves the comparator's promises (reads its arguments as `T`, total order, no panic) to its own caller, which is why it is an `unsafe fn`. The comparator must not compute `a - b`: the overflow panics in a debug build, a panic cannot leave an `extern "C"` function, and the process aborts.

</details>

## l07 · Capstone: checked foreign transaction

<details><summary>Hint 1</summary>

The foreign contract tells you which output is initialized on each status path. Reading the other one is never justified.

</details>

<details><summary>Hint 2</summary>

Allocate two MaybeUninit slots, branch on status, and copy CStr to String before returning the error. Add index context in batch using enumerate. The tests count the calls that reach the library: `score` makes exactly one call for a valid name and none for a name with a NUL, and `batch` makes no call after the first failure. Collecting an iterator of `Result`s into a `Result<Vec<_>, _>` stops at the first `Err`.

</details>

<details><summary>Check your explanation (after writing your own)</summary>

The audit runs in order. Arguments: `CString::new` rejects interior NUL *before* any foreign call. Status: only 0 is success. Output: `out` is `assume_init`ed only on success, and `error` only on failure, because each slot is initialized on exactly one path. Errors: the foreign error text is copied into an owned String immediately, because a real library may overwrite or free it on the next call (the text behind SQLite's `sqlite3_errmsg`, for example, may be overwritten by a later call on the same connection). Unwinding: nothing in the wrapper or the mock can panic across the ABI. `batch` stops at the first failure and pairs it with its index; "stops" means that no later item reaches the library, so no later call can replace the error state. A real library adds obligations that the mock does not model, and the optional labs in LABS.md meet some of them: initialization and shutdown, the matching free function for every object, threading rules that depend on the version and build options, linking, and error codes.

</details>

## R01 · Spaced review of Chapters 18, 22, and 23

<details><summary>Hint 1</summary>

A: the library writes into memory the Vec owns but has not counted yet. Recall which Vec method changes only the length, and what it requires. You cannot prepare the buffer with placeholder values, because only the library can make a `Sample`. B: a `return` inside a macro body returns from the function that *invokes* the macro. C: a struct's in-memory bytes are not a file format. Write each field's bytes yourself.

</details>
<details><summary>Hint 2</summary>

A: `Vec::with_capacity(max)`, pass `v.as_mut_ptr()` and `max`, check `n <= max`, then `unsafe { v.set_len(n) }`. Do not reach for `std::mem::zeroed()`: an all-zero `Sample` is not a valid value, and the program stops with "attempted to zero-initialize type". B: `let status: c_int = $status; if status != 0 { return Err(status); }`. C: fill a `[u8; 9]` from `kind.to_le_bytes()`, `count.to_le_bytes()`, `value.to_le_bytes()` and `write_all` it. Read with `read_exact` into a `[u8; 9]` and `from_le_bytes`. If the test reports that `read_record` or `write_record` "was still running", you wrote a loop around `read` or `write` that never ends when the call returns `Ok(0)`; `read_exact` and `write_all` turn that case into an error.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs. One misreads the library's status codes. The other returns more bytes than the message contains. Then look at every `unwrap()` and `as` in the wrapper, at the buffer each call allocates, and at what happens if the callback panics.

</details>
<details><summary>Hint 2</summary>

Success is `status == 0` and nothing else. `lib_last_error` returns the message length, so slice `&buf[..n]` (or use `CStr::from_bytes_until_nul`) from a stack `[u8; 64]`, then make one owned String. `CString::new(text).map_err(|_| "interior NUL".to_string())?`, `u16::try_from(out)`, `.cast()` instead of `as`, and `Vec::with_capacity(lib_count())` so the callback's `push` never reallocates.

</details>
<details><summary>Check your review</summary>

`parse_port` treats only negative statuses as failures, so "busy" (status 1) returns `Ok(0)` from the untouched placeholder. A zero-initialized out value hides the bug where `MaybeUninit` would have made it visible. `last_error` converts the whole 64-byte buffer, so every message ends in dozens of NUL characters. It also allocates a heap buffer per call and copies twice (`from_utf8_lossy(..).to_string()`). `CString::new(text).unwrap()` panics on caller input that should become `Err("interior NUL")`, and `out as u16` silently truncates instead of checking. `all_values` grows its Vec while C is calling back: four allocations, where the library already says how many values to expect. Reserve `lib_count()` up front. A failed allocation aborts the process; it does not panic. The supplied callback therefore has no realistic way to panic. A callback that can (indexing, `unwrap`, a closure from a user) needs `catch_unwind`, because a panic that leaves an `extern "C"` function aborts the process. None of the unsafe blocks has a SAFETY comment, and `as` casts between pointer types hide mistakes that `.cast()` makes explicit.

</details>

## Drills

Run `cargo drill ch24`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict.

<details><summary>d01 · Declaring foreign functions</summary>error (no error code): "extern blocks must be unsafe". In Rust 2024, an extern block is written `unsafe extern "C" { ... }`. The programmer is asserting that the declared signatures match the real library, and the compiler cannot check that.</details>
<details><summary>d02 · Calling a foreign function</summary>E0133. Items in an `unsafe extern` block are unsafe to call by default: the compiler cannot see the C body or its preconditions. Call it as `unsafe { abs(-5) }` with a SAFETY comment, or declare it `safe fn` if it really has no preconditions.</details>
<details><summary>d03 · Safe items in an unsafe extern block</summary>Compiles and runs. Since Rust 1.82, in every edition, an item in an `unsafe extern` block can be marked `safe fn`, and callers then need no `unsafe`. (Edition 2024 only makes the spelling `unsafe extern` mandatory.) That is a promise *you* make for every possible input, and here it is false: C's `abs(INT_MIN)` is undefined behavior, so safe code could now trigger UB. "It compiles" is not "it is sound". Keep the foreign `abs` unsafe and wrap it in a safe Rust fn that rejects `c_int::MIN`. The wrapper can keep the name `abs` if the foreign item gets another Rust name with `#[link_name = "abs"]`. Or just use `i32::abs`.</details>
<details><summary>d04 · Exporting a symbol</summary>error (no error code): "unsafe attribute used without unsafe". An unmangled symbol can collide with another symbol of the same name, which is undefined behavior at link time. Rust 2024 makes you write `#[unsafe(no_mangle)]`.</details>
<details><summary>d05 · An interior NUL</summary>Panics: `CString::new` returns `Err(NulError)` because C would treat the NUL as the end of the string, and `unwrap` panics. Replace the NUL with an ordinary byte such as a space (`"path with nul"` is still 13 bytes long, which the KEEP line checks). In real code, return the `NulError` to your caller with `?` instead of calling `unwrap`.</details>
<details><summary>d06 · A &str is not a C string</summary>E0308. `&str` is a fat pointer to UTF-8 bytes with no NUL terminator; `*const c_char` is a thin pointer to NUL-terminated bytes. Use a C string literal (`c"ferris"`) or a `CString`, and pass `.as_ptr()`.</details>
<details><summary>d07 · Callback ABIs</summary>E0308: `fn(i32) -> i32` uses the Rust ABI, and the callback type requires `extern "C"`. The ABI is part of a function pointer's type. Declare `extern "C" fn double(...)`.</details>
<details><summary>d08 · Casting a String reference</summary>E0606. `&name` points at the String's header (pointer, capacity, length), not at its bytes, so the cast is rejected. `name.as_ptr().cast::<c_char>()` points at the text. A `CString` would also guarantee the terminating NUL.</details>
<details><summary>d09 · Null function pointers</summary>E0308. Rust function pointers are never null, so `ptr::null()` (a raw pointer) cannot be one. A nullable C callback is `Option<extern "C" fn()>`, with `None` as NULL and the same size. Start with `None`, store `Some(tick)`, and take the function out before the call: `if let Some(callback) = registered { callback(); }`.</details>
<details><summary>d10 · Returning a borrowed C string</summary>E0515: `owned` is dropped at the end of `make_name`, so a `&CStr` into it would dangle. Return the owner (`CString`). It derefs to `CStr`, so the caller's `.to_bytes()` still works.</details>

## Mastery check: model answers

Open this only after you have written your own answers in NOTES.md.

<details><summary>Model answers</summary>

1. The two compilers share no type information, so each side must describe the same bytes independently. `#[repr(C)]` makes Rust lay out a struct by C's rules: fields in declaration order, each at a multiple of its alignment, the size rounded up to the struct's alignment. The `std::ffi` aliases (`c_int`, `c_long`, `c_char`, ...) are whatever the target's C compiler uses for those types, so the code stays correct on a target where `long` has 4 bytes or `char` is unsigned. `size_t` is `usize`. Pointers cross as raw pointers, which carry no lifetime and no ownership. A declaration with `extern "C"` selects the C calling rules; its parameter and return types must match the C prototype exactly, and nothing checks that they do.

2. They split ownership the same way as `String` and `&str`. `CString` owns a heap buffer that ends with a NUL and has no NUL inside; you build one to pass Rust text to C, and it must stay alive while C reads. `&CStr` borrows a NUL-terminated buffer that someone else owns, often the C library. The pointer from `as_ptr` has no lifetime in its type. It is valid only while the owner is alive and unchanged, so a pointer taken from a temporary `CString` dangles at the end of the statement. Neither type promises UTF-8: `to_str` checks, and an owned `String` needs a copy.

3. Only when the function's contract says so, which usually means "on the success status". Before the call the slot is `MaybeUninit`. After a failure the wrapper reads nothing from the result slot: it may still be uninitialized, and reading it is undefined behavior. What the failure path may read is whatever the contract says is written on failure, for example an error slot or the library's last-error message.

4. The allocator that created the object, through the library's own free function. Memory from C's `malloc` never goes to `Box::from_raw`, and Rust memory never goes to C's `free`. The wrapper is a struct with a private `NonNull` handle and a `Drop` implementation that calls the free function. It has no `Clone` and no `Copy`, so exactly one owner exists and `Drop` runs once. Because the handle is wrapped as soon as it is created, an early return with `?` or a panic in later Rust code still drops the wrapper and frees the object.

5. The borrowed object is a struct that holds the raw pointer and a `PhantomData<&'a Owner>`. A constructor of the form `fn child(&self) -> Child<'_>` copies the lifetime of the borrow of the owner into the child. From then on the borrow checker treats the child as a reference to the owner: the owner cannot be dropped, moved or changed while the child is in use. The marker takes no space at run time.

6. For as long as the library may call the callback. For a synchronous library that is the duration of one call, and a local variable is enough. For a library that stores the callback, the userdata must live until it is unregistered, usually as a `Box` turned into a raw pointer and taken back afterwards. A panic that reaches the edge of an `extern "C"` function aborts the process (since Rust 1.81); it never unwinds into C. So a callback either cannot panic, or catches the panic with `catch_unwind` and reports an error code.

7. Nobody checks it: the compiler cannot read C headers, and the linker matches names without types. `unsafe extern` is the author's promise that every declaration in the block matches the real symbol. `safe fn` inside the block is a further promise that no argument value can cause undefined behavior, so callers need no `unsafe`. An `unsafe { ... }` block around a call is the caller's promise that the preconditions of that one call hold, and the `// SAFETY:` comment says why.

8. Whether the library must be initialized before first use and shut down after last use, and whether those calls are counted. Which functions may be called from which threads, and whether a handle may move between threads; without proof, the wrapper is neither `Send` nor `Sync`. Where error text lives, and which later call replaces it, so that the wrapper knows when to copy. Which versions of the library the declarations match. How the library is found and linked (system library, static or dynamic, link flags). For every pointer: who owns it, whether it may be null, until when it is valid, and which function frees it.

</details>
