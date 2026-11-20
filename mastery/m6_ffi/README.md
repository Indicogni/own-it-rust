# Mastery M6 — FFI across a real language boundary

[All chapters](../../README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

Chapter 24 taught the rules of the foreign function interface with a "foreign" side that was written in Rust. In this module the other side is real. A build script compiles a C library with your system's C compiler. You write its bindings by hand and check them against that compiler, wrap the library in a safe Rust type, pass closures into it, and move strings and buffers through it. Then you turn around: you export a Rust library with a C header, and a C program that the tests compile and run calls your code. The capstone does both at once.

**Who it is for.** You have finished the chapters, in particular chapters [23](../../exercises/chapter23/README.md) and [24](../../exercises/chapter24/README.md). You can write a small `unsafe` function and its SAFETY comment. Now you want to be the person who can bind a C library for a team, or ship a Rust library to C users, and defend every `unsafe` block in a review.

**Before you start:**

| Chapter | What this module uses from it |
| --- | --- |
| [5](../../exercises/chapter05/README.md) | References and lifetimes: a borrowed view that cannot outlive its owner |
| [8](../../exercises/chapter08/README.md) | Packages, features, dependencies, build profiles |
| [13](../../exercises/chapter13/README.md) | `Drop`, and types that own a resource |
| [14](../../exercises/chapter14/README.md) | Closures and the `Fn` traits |
| [17](../../exercises/chapter17/README.md) | Strings, UTF-8, bytes |
| [19](../../exercises/chapter19/README.md) | `Send` and `Sync` as a user |
| [23](../../exercises/chapter23/README.md) | Raw pointers, `unsafe fn`, SAFETY comments, `MaybeUninit`, a first Miri run |
| [24](../../exercises/chapter24/README.md) | All of it: `repr(C)`, `CStr`/`CString`, `unsafe extern`, status codes, callbacks, opaque handles |

You must be able to read simple C (declarations, pointers, structs, `malloc` and `free`). You do not need to write any.

**Time:** about 27 hours in total, as a rough estimate; [CONCEPTS.md](CONCEPTS.md) has an estimate for each lesson.

## Setup

This is a standalone Cargo package. It is not part of the root package, so the chapter checks never need a C compiler. Rust 1.99.0 is pinned in `rust-toolchain.toml`, identical to the root one. The only crate it uses is [`cc`](https://docs.rs/cc) (pinned with `=`), in the build script and in the tests; the lesson code itself uses only the standard library.

**You need a C compiler.** The build script finds it through the `cc` crate.

| System | What to install | Check |
| --- | --- | --- |
| Linux | `gcc` or `clang` (for example the package `build-essential`) | `cc --version` |
| macOS | the Xcode command line tools: `xcode-select --install` | `cc --version` |
| Windows (MSVC toolchain) | the "Desktop development with C++" workload of the Visual Studio Build Tools; Rust's MSVC toolchain needs it anyway | a Rust "hello world" builds |

Run every command from the root of the course repository:

```sh
# The first build downloads the pinned `cc` crate; afterwards you can add --offline.
cargo test --manifest-path mastery/m6_ffi/Cargo.toml --locked --all-targets --no-run
cargo test --manifest-path mastery/m6_ffi/Cargo.toml --locked --release --all-targets --no-run
```

The starters compile. Tests fail at `todo!()` and `guess()` until you fill them in; the tests of lesson 07 and of the capstone fail with messages from the C compiler until you have written a header and the functions behind it. That is the starting state, not a broken setup.

## What is in the package

```text
mastery/m6_ffi/
  Cargo.toml              links = "kvtab", crate-type = ["lib", "staticlib"]
  build.rs                compiles the C code; the worked example of lesson 02
  vendor/kvtab/           the C library: kvtab.h, kvtab.c (MIT), and what bindgen generated
  csrc/layout_probe.c     C code that reports sizes, offsets and constants (lesson 03)
  src/
    l01_libc.rs ... l08_capstone.rs, c01_code_review.rs    the lessons
    sys.rs                reference bindings to kvtab (the answer to lesson 03; used from lesson 04 on)
    c_reference.rs        reference declarations for lesson 01
    model.rs              a Rust model of kvtab, compiled only under Miri
    probe.rs, support.rs, harness.rs                       support code for the tests
  sys_lab/                lesson 02's lab: a `-sys` crate that you finish, and its checker
  include/                the two C headers that you write (lesson 07, capstone)
  ctests/                 the C test programs for lesson 07 and the capstone
  tests/miri_export.rs    the same two scenarios, called from Rust under Miri
```

`kvtab` is a small key-value table: NUL-terminated keys, byte values, a fixed capacity, insertion order. Read `vendor/kvtab/kvtab.h` early. It is the documentation that every SAFETY comment from lesson 03 on has to point to.

## Work through the module

For each lesson:

1. **Read** the lesson's section in [CONCEPTS.md](CONCEPTS.md), then the header of the lesson file.
2. **Predict.** Answer the `p_predictions` test first: replace each `guess()` with a value. The prediction tests do not need your implementation.
3. **Implement.** Lessons 01 to 03 give you signatures. Lessons 04 to 06 give you the public signatures of a safe wrapper; its representation, its `Drop`, its thread-safety and every SAFETY argument are yours. Lesson 07 and the capstone give you a contract in prose and a C test program; you design every Rust item and write the C header.
4. **Check technique.** Some tests check *how*: the C table is freed exactly once (lesson 04), a panic is carried over the C frames instead of aborting (05), `peek` returns the library's own bytes (06), a changed C file is really recompiled (02). Then run clippy; this package warns about every `unsafe` block without a SAFETY comment.
5. **Run Miri** where the lesson says so (see below).
6. **Explain** the EXPLAIN prompt in [NOTES.md](NOTES.md), add one edge-case test, then compare with *Check your explanation* in [HINTS.md](HINTS.md).

Lessons 05 and 06 add methods to the `Table` of lesson 04, and the capstone can use all of them, so do the lessons in order.

| Done | Lesson | What you do |
| --- | --- | --- |
| [ ] | [01 · The C library again](src/l01_libc.rs) | `strtol` with its end pointer and `errno`, `div` returning a struct, `bsearch` returning a pointer into your slice |
| [ ] | [02 · Building C from Cargo](src/l02_sys_crate.rs) | Finish a `-sys` crate in [`sys_lab/`](sys_lab/): the `links` key, a build script with `cc`, a feature that defines a C macro, the declarations |
| [ ] | [03 · Bindings by hand](src/l03_bindings.rs) | Translate `kvtab.h`: constants, a struct, callback types, twelve functions; the C compiler checks your layout |
| [ ] | [04 · A safe wrapper](src/l04_safe_wrapper.rs) | `Table`: an invariant, `Drop`, error mapping, `into_raw`/`from_raw`, `Send` but not `Sync` |
| [ ] | [05 · Callbacks](src/l05_callbacks.rs) | `for_each` with a closure on the stack, an observer closure that the C table owns, panics at the boundary |
| [ ] | [06 · Strings and buffers](src/l06_strings_buffers.rs) | A static string, a caller's buffer, the two-call protocol, a borrowed view with a lifetime, `&str` keys, UTF-16 |
| [ ] | [C01 · Code review](src/c01_code_review.rs) | A teammate's wrapper that works on the happy path and is unsound |
| [ ] | [07 · Exporting a Rust library to C](src/l07_export.rs) | A word tally with a C interface, its header [`include/m6_tally.h`](include/m6_tally.h), tested by a C program |
| [ ] | [08 · Capstone](src/l08_capstone.rs) | A stock list: C program → your Rust library → the C table, with a C callback, a caller's buffer, a thread-local error text and a panic guard |
| [ ] | [Drills](../../drills/m6/) | Ten programs: predict the compiler's or the program's verdict, then fix |

```sh
cargo test --manifest-path mastery/m6_ffi/Cargo.toml -- --list
cargo test --manifest-path mastery/m6_ffi/Cargo.toml l01_
cargo test --manifest-path mastery/m6_ffi/Cargo.toml --release l01_
cargo test --manifest-path mastery/m6_ffi/Cargo.toml l07_ -- --nocapture
cargo test --manifest-path mastery/m6_ffi/Cargo.toml --doc
cargo clippy --manifest-path mastery/m6_ffi/Cargo.toml --all-targets
cargo fmt --manifest-path mastery/m6_ffi/Cargo.toml --check
cargo drill m6
```

Lesson filters (`l01_` … `l08_`, `c01_`) match the module path of the tests. `cargo test --doc` compiles and runs every `rust` block of CONCEPTS.md. In the starting state clippy has two kinds of warnings that are part of the work: missing SAFETY comments in C01, and `drop(table)` calls in the tests that do nothing until your `Table` has a `Drop` impl (lesson 04).

The tests of lessons 02, 07 and 08 start Cargo and the C compiler themselves. They put everything they build into `m6-nested/` inside this package's target directory, so nothing is written into the source tree. The first such run is slower, because it builds the `cc` crate and the static library a second time.

### When the whole test program dies

In this module a mistake in an `unsafe` block is a real memory error in a real process. Most mistakes fail one test with a message. Some end the test program instead: you see `signal: 11, SIGSEGV`, `signal: 6, SIGABRT`, or on Windows an exit code such as `0xc0000005`, and no test summary. That is information too. Run the lesson's tests one at a time (`cargo test ... l04_ -- --test-threads=1`) to find the test that does not come back, then run the same test under Miri, which stops at the first invalid access and says what was wrong with it. Two tests provoke an abort on purpose when a panic is not handled (in lesson 05 and in C01); they run that part in a child process and report it as an ordinary failure.

### What the tests of lesson 07 and the capstone do

1. Build this package as a static library: `cargo rustc --lib --crate-type staticlib -- --print native-static-libs`. The last flag makes rustc print the system libraries that the final link needs.
2. Compile `ctests/<name>.c` with the system's C compiler (found by the `cc` crate), with `include/` and `vendor/kvtab/` on the include path.
3. Link the C object with the static library and those system libraries.
4. Run the program and check its exit status and its output.

[`src/harness.rs`](src/harness.rs) is that procedure as code; read it after lesson 07. After `cargo build`, the static library is also in the target directory (`debug/libm6_ffi.a`, or `debug\m6_ffi.lib` on Windows), if you want to link a C program of your own.

### Miri

Miri interprets Rust code and checks every memory access. It cannot run compiled C. This package uses it in three ways.

- **Lessons 03 to 06 and C01** run under Miri against [`src/model.rs`](src/model.rs), a Rust model of kvtab that follows the contract in `kvtab.h`. When the crate is built for Miri, the model defines the symbols `kvtab_new`, `kvtab_put` and so on, and the bindings resolve to it. Miri then checks **your side** of the boundary: a table that is never freed, a table freed twice, a view that outlives the bytes behind it, a buffer that is too short. It proves nothing about `kvtab.c`.
- **Lesson 07 and the capstone** have [`tests/miri_export.rs`](tests/miri_export.rs): the scenario of the C test program, written in Rust, calling your exported functions through `unsafe extern "C"` declarations. Miri also reports a definition whose types differ from the declaration, a string or a panic payload that is never released, and (with a second thread, which the C program does not have) an error text that is shared between threads.
- **Lesson 01, lesson 02 and the tests that compile C** are skipped under Miri (`ignored`): they need real C code, Cargo or a C compiler.

```sh
rustup toolchain install nightly-2026-11-20 --component miri --component rust-src     # once
cargo +nightly-2026-11-20 miri test --manifest-path mastery/m6_ffi/Cargo.toml --lib l04_
cargo +nightly-2026-11-20 miri test --manifest-path mastery/m6_ffi/Cargo.toml --lib
cargo +nightly-2026-11-20 miri test --manifest-path mastery/m6_ffi/Cargo.toml --test miri_export
```

Use `--lib` or `--test`, as shown. Without them Miri also runs the documentation tests, and the first example of CONCEPTS.md calls the real C library.

### Platforms

The package is written for Linux, macOS and Windows with the MSVC toolchain. The C code is standard C without POSIX functions, lesson 01 uses only functions that all three C libraries have, and the build and link steps go through the `cc` crate and through what rustc reports, with no hand-written list of system libraries. The predictions about sizes and offsets assume a 64-bit target, and say so. The package was developed and verified on macOS (64-bit ARM). If a step fails on your platform, the failure message of the test shows the full compiler or linker output; `src/harness.rs` is short enough to adjust.

## Topic map

| Topic | Where |
| --- | --- |
| Platform types: `c_int`, `c_long`, `c_char`, `size_t` | Lesson 01; CONCEPTS 01 |
| Out-pointers and `MaybeUninit` | Lessons 01, 03, 04; drill 03 |
| `errno` and `link_name` | Lesson 01 |
| A struct returned by value; `repr(C)` layout and padding | Lessons 01, 03; drill 07 |
| A precondition carried by a type | Lesson 01 (`SortedInts`) |
| Build scripts, the `cc` crate, `OUT_DIR`, `rerun-if-changed` | Lesson 02; `build.rs` |
| `links`, build metadata, `DEP_*`, the `-sys` convention | Lesson 02 |
| A Cargo feature that selects C code | Lesson 02 |
| Translating a header; opaque types; enums as constants | Lesson 03; `src/sys.rs` |
| Checking bindings: the C compiler, `clashing_extern_declarations`, bindgen, Miri | Lessons 01, 03; CONCEPTS 03 |
| What hand-written bindings must handle beyond kvtab: macros, inline functions, unions, bit-fields, packed structs, C++ | CONCEPTS 03 (text only) |
| ABI strings: `"C"`, `"system"`, `"C-unwind"`; variadic functions | CONCEPTS 01 (text only); lesson 05; drill 08 |
| Ownership of a handle: `NonNull`, `Drop`, `into_raw`, `from_raw` | Lesson 04; drill 06 |
| Mapping status codes without losing any | Lesson 04; drill 02 |
| `Send` and `Sync` for a wrapper, from the library's documentation | Lesson 04, C01; drill 04 |
| Callbacks with userdata; generic trampolines | Lesson 05; drill 01 |
| A closure that C owns, and its drop function | Lesson 05 |
| Panics at the boundary: catch and resume, abort, `"C-unwind"` | Lesson 05, capstone; drill 08 |
| `CStr`, `CString`, `&str`, bytes, UTF-16 | Lesson 06 |
| The caller's buffer and the two-call protocol | Lesson 06, capstone |
| A borrowed view of foreign memory with a lifetime | Lesson 06; drill 05 |
| `extern "C"` definitions, `#[unsafe(no_mangle)]`, symbol names, what `pub` does and does not do | Lesson 07; drill 10 |
| Crate types: `lib`, `staticlib`, `cdylib`; `native-static-libs` | Lesson 07; `src/harness.rs` |
| Writing a C header; cbindgen | Lesson 07, capstone; CONCEPTS 07 |
| Returning a string to C; who frees | Lesson 07 |
| ABI stability; opaque handles; nullable function pointers | Lesson 07; CONCEPTS 07; drill 09 |
| A guard at every entry point; a thread-local error text | Capstone; CONCEPTS 08 |
| A Rust model of a C library for Miri | `src/model.rs` |

## Mastery check

Answer these from memory, in [NOTES.md](NOTES.md). Model answers are at the end of [HINTS.md](HINTS.md).

1. What exactly does an `unsafe extern "C"` block promise, and to whom? Name three independent ways to check a hand-written binding.
2. Describe what happens between `cargo build` and the moment rustc compiles your crate, for a package whose build script uses `cc`. What do `links` and `cargo::metadata` do, and why must a build script name its C inputs?
3. Why does a binding translate a C `enum` to integer constants and a C function pointer to an `Option`? How is an opaque struct declared, and what does each part of the declaration prevent?
4. State the invariant of a type that owns a C handle. How do `Drop`, `into_raw` and `from_raw` relate to it? What evidence do you need before you write `unsafe impl Send`, and what more for `Sync`?
5. How does a Rust closure reach a C callback? Compare the callback that runs during one call with the one that C keeps: where the closure lives, who frees it, which bounds it needs. What are the three ways to deal with a panic in it?
6. Which three questions must be answered for every pointer that crosses the boundary? Why is `fn peek(&self) -> Option<&[u8]>` a sound signature for "valid until the next put", and what would make it unsound?
7. What makes a Rust function callable from C, and what does each crate type produce? Who checks that the header matches the Rust code? Which later changes break existing C programs silently, and which are safe?
8. What must a guard at an exported function do, what can still go wrong inside it, and how does the C caller find out what happened?

Redo the capstone from its contract in a later session, without looking at your first solution. Then change one requirement (for example: quantities may be negative, or the report is sorted by quantity), and say which parts of the header, the Rust code and the C test program must change with it.

## Solutions

Model solutions are in [solutions/mastery/m6_ffi/](../../solutions/mastery/m6_ffi/), at the same paths as the files they replace: one file per lesson with the predictions filled in, the three files of the lesson 02 lab, and the two headers. [solutions/drills/m6/](../../solutions/drills/m6/) has each drill repaired and the expected outcome of every drill. Use them after an honest attempt and after the hints: compare your solution with the model, and note in NOTES.md what differs and why. For the lessons with a free design (04 to 08) your solution may differ from the model and still be correct; what must agree are the invariants and the SAFETY arguments. The reference bindings in `src/sys.rs` are the model answer to lesson 03's declarations.

## Validation scope

Use the pinned Rust 1.99.0 toolchain and run commands from the root of the course repository. `cargo test --manifest-path mastery/m6_ffi/Cargo.toml --locked --all-targets --no-run` checks that the starters compile, in debug and with `--release`. The solved package passes all tests in both profiles, is clean under `cargo clippy --all-targets`, and passes `cargo +nightly-2026-11-20 miri test --lib` and `--test miri_export` (with the tests that need real C code ignored, as described above). The drills are checked with `rustc --edition 2024 -A warnings`. The outputs of `bindgen` 0.73.2 and `cbindgen` 0.29.4 that are shown in CONCEPTS.md and in `vendor/kvtab/bindgen_reference.rs` were produced by those tools; neither tool is needed to build or test this package.
