# Optional labs: real foreign libraries

[Chapter guide](README.md) · [Concepts](CONCEPTS.md)

These two projects are **optional**. No test in the course checks them. The numbered lessons use foreign-style functions that are written in Rust, and lesson 08 calls the C library that every Rust program already links. The labs go one step further: you compile C code yourself, link a library that is not linked by default, and export a Rust function to C.

Work in a new directory outside the course repository. Both labs need a C compiler. Lab 2 also needs the SQLite library and its header file on your machine.

The skeletons below (the two `Cargo.toml` files and the two `build.rs` files) were built and run on macOS with the course toolchain. Everything after the skeleton is your own work, and details differ between platforms. Write down what you had to change on yours.

For a larger guided project on the same subject, see the mastery module [`mastery/m6_ffi`](../../mastery/m6_ffi/README.md).

## Lab 1 · A local C library, and a Rust function that C calls

Goal: link C code that you wrote into a Rust program, export a Rust function back to C, and keep panics from crossing the boundary.

`Cargo.toml` (the `cc` crate compiles C files from a build script; check its documentation for the current version):

```toml
[package]
name = "c-ffi-lab"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["rlib", "staticlib"]  # staticlib is for the C driver in part B

[build-dependencies]
cc = "1"
```

```rust
// build.rs
fn main() {
    cc::Build::new().file("c/lab.c").compile("lab");
    println!("cargo::rerun-if-changed=c/lab.c");
    println!("cargo::rerun-if-changed=c/lab.h");
}
```

### Part A: call C from Rust

1. Create the package with the two files above, an empty `src/lib.rs` and an empty `c/lab.c`.
   *Checkpoint:* `cargo build` compiles the C file and links it.
2. Write `c/lab.h` and `c/lab.c`, a small C library with:
   - a function that changes a record whose layout matches a Rust `#[repr(C)]` struct;
   - a division that returns a status and writes its result through an output pointer;
   - a function that takes a callback and a `void *` userdata;
   - a read-only variable that holds the library version.
   Use exact C integer types and `size_t` in the header.
3. Translate each type to its `std::ffi` alias, and `size_t` to `usize`. Print `sizeof`, `_Alignof` and `offsetof` for the record in C, and `size_of`, `align_of` and `offset_of!` in Rust. Compare the numbers.
   *Checkpoint:* the C and Rust numbers agree for every field of the record.
4. Declare the functions and the version variable in an `unsafe extern "C"` block. A variable is declared with `static NAME: Type;`. Write safe wrappers, and explain every precondition in a `// SAFETY:` comment. Do not use mutable global state.
5. Through the safe wrappers, exercise: an owned `CString` argument, a borrowed `CStr` result, a null pointer, reading the output only on success, and an allocate function with its matching free function.
   *Checkpoint:* the division wrapper returns `Err` on failure without reading the output slot, and the callback sees the userdata that you passed.

### Part B: call Rust from C

6. Add a Rust function with `#[unsafe(no_mangle)]` and `extern "C"`. Give it a name that no other library is likely to use.
7. Build the package and call the function from a tiny C program that you link against the static library in `target/debug/`. On Linux the C compiler may also need the system libraries that `cargo rustc -- --print native-static-libs` lists.
   *Checkpoint:* the C program prints a value that your Rust function computed.
8. Do not let a Rust panic unwind into C. Check inputs without panicking. For a callback that runs a closure supplied by a user, write a trampoline: an `extern "C"` function that takes the closure from the userdata pointer, runs it inside `std::panic::catch_unwind`, records a failure in the userdata, and returns a C error code.
9. Note in your write-up that a failed allocation aborts the process, and that with `panic = "abort"` there is nothing for `catch_unwind` to catch.
10. Explain the linker flags and the library search paths that your build used. `cargo build -vv` shows them.

Never free memory from C's `malloc` with `Box::from_raw`, and never free Rust memory with C's `free`.

Acceptance criteria:

- [ ] Success, failure and null cases for each wrapper.
- [ ] The userdata lifetime of the callback is shown in a test and explained.
- [ ] A panicking closure becomes a C error code, not an abort.
- [ ] The output of the C program is recorded.
- [ ] The layout numbers from C and from Rust agree.
- [ ] A written explanation of the linker flags, the search paths and the two abort cases.

## Lab 2 · A raw and a safe interface to SQLite

Goal: bind a real C library that has its own allocator, its own error state and child objects that depend on a parent object. SQLite is a good subject: it is one library with a stable C interface, it is installed on many systems, and an in-memory database needs no file and no network.

Prerequisites: the SQLite library (`libsqlite3`) and its header `sqlite3.h`. Do not assume that they are installed; check first, and do not change system packages unless you decide to. Record the version that you build against, and read the official documentation for that version at <https://www.sqlite.org/c3ref/intro.html>. Use only the database name `:memory:` or a file in a temporary directory that you create.

```toml
[package]
name = "sqlite-ffi-lab"
version = "0.1.0"
edition = "2024"
```

```rust
// build.rs: asks the linker for the system's SQLite library.
fn main() {
    println!("cargo::rustc-link-lib=sqlite3");
}
```

*Checkpoint:* a test that calls `sqlite3_libversion()` passes, and you have written down the version string.

### Stage 1: the raw interface

1. Declare the exact signatures of `sqlite3_libversion`, `sqlite3_open`, `sqlite3_close`, `sqlite3_errmsg`, `sqlite3_prepare_v2`, `sqlite3_step`, `sqlite3_column_int64`, `sqlite3_column_text` and `sqlite3_finalize`. Copy the prototypes from `sqlite3.h` into comments, as in lesson 08. Use an opaque Rust type for the connection and another for the statement.
   *Checkpoint:* opening `:memory:` returns status 0 and a handle that is not null.
2. Read what `sqlite3_open` does to its out-parameter when it fails. On this point SQLite differs from the contract in lesson 03: a failed open usually still hands you a handle, and you must close it.
   *Checkpoint:* opening a path inside a directory that does not exist returns a non-zero status, and your code closes the handle that came back.
3. `sqlite3_errmsg` lends you a string that the next call on the same connection may overwrite. Copy it into a `String` at once.
   *Checkpoint:* preparing invalid SQL gives you an owned Rust error that contains SQLite's message, and the error is still correct after further calls.

### Stage 2: the safe interface

4. Write a `Connection` that owns the connection handle and closes it in `Drop`. Cleanup must not panic.
5. Write a `Statement<'conn>` that cannot outlive its `Connection`. SQLite refuses to close a connection that still has statements (the close call returns `SQLITE_BUSY`), so the Rust lifetime must make that situation impossible.
   *Checkpoint:* a program that uses a statement after its connection was dropped does not compile.
6. `sqlite3_column_text` lends you a pointer that is valid only until the next step, reset or finalize of that statement. Either copy the text, or return a borrow whose lifetime ends at the next `&mut self` call on the statement.
7. Run `SELECT 6 * 7, 'text'` through the raw calls and through the safe types, and compare the two programs.
8. Return an error from your own Rust code after the connection and a statement exist, and test that both are still cleaned up.
9. Do not write `unsafe impl Send` or `unsafe impl Sync` for your types unless you can prove them from the documentation of the threading mode that your library was built with (`sqlite3_threadsafe()` reports it).

Acceptance criteria:

- [ ] Opening `:memory:` and running one query.
- [ ] A failed open, with the copied message and no leaked handle.
- [ ] SQL text with an interior NUL, rejected before any foreign call.
- [ ] A failed prepare that never reads an uninitialized statement pointer.
- [ ] A test of the drop order of statement and connection.
- [ ] A compile-fail example of a statement that outlives its connection.
- [ ] An ownership table with one row for each pointer: borrowed or owned, may it be null, until when is it valid, which function frees it, which thread may use it.
- [ ] Cleanup shown to run after the Rust-side error of step 8.
- [ ] A written explanation of why a safe Rust lifetime does not, by itself, make a foreign object safe to use from several threads.

In your write-up, record the toolchain, the target, the SQLite version, the link flags, and anything that did not work on your platform.

References: [Rustonomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html), [`std::ffi`](https://doc.rust-lang.org/std/ffi/), [build scripts in the Cargo book](https://doc.rust-lang.org/cargo/reference/build-scripts.html), [the `cc` crate](https://docs.rs/cc/latest/cc/), and [the SQLite C interface](https://www.sqlite.org/c3ref/intro.html).
