# Mastery M6 concepts — FFI across a real language boundary

[Module guide](README.md) · [Hints](HINTS.md) · [Notes](NOTES.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. Every `rust` block in this file was compiled and
run with the course toolchain (Rust 1.99.0, edition 2024), and every block
except the first one, which calls the real C library, was also run under Miri.
Every compiler, Cargo and linker message was copied from a real run on that
toolchain and shortened: a line with `...` marks a cut, and the end of a message is often
left out. Linker messages and the output of `bindgen` and `cbindgen` come from
one machine (macOS on a 64-bit ARM processor); where another platform prints
something else, the text says so. File names in messages are written as
`src/main.rs`.

## What you will be able to do

After this module you can:

- call functions of the C library that return results through pointers,
  through `errno` and as structs, and name the precondition of every call;
- compile C code from a Cargo build script, link it, and lay out a `-sys`
  crate that other packages can depend on;
- translate a C header into Rust declarations by hand, and check the
  translation against the C compiler;
- wrap a C library in a safe Rust type, with an invariant that justifies each
  `unsafe` block, and decide from the library's documentation whether the type
  may be `Send` or `Sync`;
- pass Rust closures to C as callbacks, both for the length of one call and
  for as long as C keeps them, and keep panics out of C frames;
- move strings and buffers across the boundary in both directions and say, for
  each pointer, who frees it and how long it is valid;
- export a Rust library to C: a static library, a header, and a C program that
  uses both.

## Before you start

All of Chapter 24: `repr(C)`, `CStr` and `CString`,
`unsafe extern` blocks, status codes and out-parameters, callbacks with
userdata, opaque handles. All of Chapter 23: raw pointers, SAFETY comments,
`MaybeUninit`. Chapter 8 for Cargo packages, features and build profiles.
Chapter 13 for `Drop`. Chapter 19 for `Send` and `Sync` as a user. You must be
able to read simple C: declarations, pointers, structs and `malloc`/`free`.
You do not need to write C.

You also need a C compiler: `cc` (gcc or clang) on Linux and macOS, the MSVC
build tools on Windows. The [module guide](README.md) says how to check.

## Time

These are estimates for a careful learner, including the
reading:

| Lesson | Estimate |
| --- | --- |
| 01 · The C library again | 2 hours |
| 02 · Building C from Cargo | 2.5 hours |
| 03 · Bindings by hand | 2.5 hours |
| 04 · A safe wrapper | 3 hours |
| 05 · Callbacks | 3 hours |
| 06 · Strings and buffers | 3 hours |
| 07 · Exporting a Rust library to C | 3 hours |
| C01 · Code review | 1.5 hours |
| 08 · Capstone | 5 hours |
| Drills | 1 hour |

## Words used in every section

- The *boundary* is the place where a call goes from code compiled by one
  compiler to code compiled by another. Neither compiler sees the other side.
- A *precondition* of a function is something that must be true before the
  call. If it is not, the function's behavior is not defined.
- A function is *sound* if no caller can cause undefined behavior with it
  without writing `unsafe` (and without breaking a documented `# Safety`
  rule). A safe function must be sound for every argument.
- An *invariant* of a type is a statement about its fields that is true
  between any two calls of its methods. Safe code cannot break it, because
  the fields are private.
- A pointer is *owned* by whoever must release the memory behind it, and
  *borrowed* by everybody else, who may use it only for a stated time.

One rule runs through the whole module. For every precondition of a foreign
function, a safe wrapper must do one of three things: **check** it at run time,
**guarantee** it by construction (usually through a type), or **pass it on** to
its own caller by being an `unsafe fn` with a `# Safety` section. There is no
fourth option, and "it works in practice" is not one of the three.

## Lesson 01 · The C library again: out-pointers, errno, a struct by value, a pointer into your slice

**Assumed:** Chapter 24, lesson 08 (calling `strlen`, `abs` and `qsort`).

### The idea

An `unsafe extern "C"` block is a promise that you make to the compiler: "a
function with this name and exactly this signature exists, and the linker will
find it." The compiler believes you. It generates the call from your
declaration and never sees the C header. The word `unsafe` on the block marks
that promise.

Chapter 24 used three simple functions. Real C interfaces use four more shapes,
and this lesson has one function for each.

**1. Types that differ between platforms.** C's `long` has 64 bits on 64-bit
Linux and macOS and 32 bits on 64-bit Windows. `char` is signed on some
processors and unsigned on others. Rust's `i64` and `i8` do not change, so they
are the wrong translation. The aliases in `std::ffi` follow the platform:

| C type | Rust type | Notes |
| --- | --- | --- |
| `int`, `unsigned int` | `c_int`, `c_uint` | 32 bits on the platforms of this course |
| `long`, `unsigned long` | `c_long`, `c_ulong` | 32 bits on Windows, 64 bits on 64-bit Linux and macOS |
| `char` | `c_char` | `i8` or `u8`, depending on the target |
| `size_t` | `usize` | |
| `double` | `f64` | |
| `void *`, `const void *` | `*mut c_void`, `*const c_void` | |

The string after `extern` names the calling convention. `"C"` is the C
convention of the platform, and it is what almost every library uses.
`"system"` is the convention of the operating system's own API. It differs
from `"C"` only on 32-bit Windows, where the Windows API uses another
convention (`stdcall`); declarations of Windows API functions use it.
`"C-unwind"` belongs to lesson 05.

**2. A second result through a pointer.** C has one return value. A function
with two results takes the address of a variable and writes the second result
there. The parameter is called an *out-pointer*.

**3. An error reported on the side.** Many C library functions report failure
by setting `errno`, a per-thread integer. `errno` is a macro, not a variable:
there is no linker symbol with that name. Behind the macro each C library has
a function that returns the address of the calling thread's error number:

```text
platform            the macro `errno` expands to
Linux               (*__errno_location())
macOS, FreeBSD      (*__error())
Windows             (*_errno())
```

The lesson file declares that function once, with a `link_name` attribute for
each platform, so that Rust code has one name for it. Two rules come with
`errno`. A function that succeeds does not set it to 0, so you set it to 0
yourself before the call. And any later call into the C library may change it,
so you read it directly after the call.

**4. A struct by value, and a pointer into your own memory.** A small struct
is returned in registers. Which registers depends on the fields, so the Rust
type must be `#[repr(C)]` with the same fields in the same order. A function
that searches your array returns a pointer to an element; you turn it back
into an index with pointer subtraction.

### Worked example: `frexp` and `memchr`

`frexp` splits a floating-point number into a fraction and a power of two. The
fraction is the return value, and the exponent comes back through an
out-pointer. `memchr` searches bytes and returns a pointer to the first match.

```rust
use std::ffi::{c_int, c_void};

// SAFETY: both prototypes are those of the C standard library:
//     double frexp(double value, int *exponent);
//     void  *memchr(const void *block, int byte, size_t len);
unsafe extern "C" {
    fn frexp(value: f64, exponent: *mut c_int) -> f64;
    fn memchr(block: *const c_void, byte: c_int, len: usize) -> *mut c_void;
}

/// Returns `(fraction, exponent)` with `value == fraction * 2^exponent`.
fn split(value: f64) -> (f64, c_int) {
    let mut exponent: c_int = 0;
    // SAFETY: `&mut exponent` is valid for one write of an int, and frexp
    // keeps no pointer. Every f64 is an allowed argument.
    let fraction = unsafe { frexp(value, &mut exponent) };
    (fraction, exponent)
}

/// The index of the first `needle` in `haystack`.
fn position(haystack: &[u8], needle: u8) -> Option<usize> {
    if haystack.is_empty() {
        // The pointer of an empty slice does not point to an object, and C
        // wants a valid pointer even when the length is 0.
        return None;
    }
    let start = haystack.as_ptr();
    // SAFETY: `start` is readable for `haystack.len()` bytes, which is the
    // length we pass. memchr only reads and keeps no pointer.
    let found = unsafe { memchr(start.cast(), c_int::from(needle), haystack.len()) };
    if found.is_null() {
        return None;
    }
    // SAFETY: a non-null result points to one of the bytes of `haystack`:
    // same allocation as `start`, and not before it.
    Some(unsafe { found.cast_const().cast::<u8>().offset_from_unsigned(start) })
}

fn main() {
    assert_eq!(split(8.0), (0.5, 4)); // 8 = 0.5 * 2^4
    assert_eq!(split(0.75), (0.75, 0));
    assert_eq!(position(b"key=value", b'='), Some(3));
    assert_eq!(position(b"no equals sign", b'='), None);
    assert_eq!(position(b"", b'='), None);
}
```

```text
position(b"key=value", b'='):

   start                 found
     |                     |
     v                     v
   +---+---+---+---+---+---+---+---+---+
   | k | e | y | = | v | a | l | u | e |      the slice's own memory
   +---+---+---+---+---+---+---+---+---+
     0   1   2   3                            found - start = 3
```

Note what `split` did for the out-pointer: `exponent` is initialised before
the call. Then reading it afterwards is defined even on a path where C did not
write it. Lesson 03 uses `MaybeUninit` for the cases where initialising first
is not possible.

### Preconditions that a type can carry

`position` has no precondition left for its caller: every slice works. Compare
`bsearch`, which this lesson uses. C requires that the array is sorted in the
order of the comparison function, and does not say what happens otherwise. A
caller can break that rule with safe code alone, by passing an unsorted slice.

The three options from the start of this file apply. *Checking* on each call
costs more than the search itself. *Passing it on* makes the function
`unsafe`. The third option is to *guarantee* it: a type whose constructor
checks the order once, with a private field so that no other code can make a
value. Every method of that type may then rely on the order. This is the same
idea as `str`: a `&str` is a `&[u8]` that is known to be UTF-8.

### Mistakes people make

**A declaration that does not match the library.** Nothing stops this from
compiling. If the crate happens to contain two declarations of the same
symbol, the compiler compares them and warns. The lesson uses that warning as
a checking tool (with `deny`, so that it becomes an error):

```text
warning: `strlen` redeclared with a different signature
  --> src/main.rs:13:9
   |
 6 |         pub fn strlen(text: *const c_char) -> usize;
   |         -------------------------------------------- `strlen` previously declared here
...
13 |         pub fn strlen(text: *const c_char) -> c_int;
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ this signature doesn't match the previous declaration
   |
   = note: expected `unsafe extern "C" fn(*const i8) -> usize`
              found `unsafe extern "C" fn(*const i8) -> i32`
   = note: `#[warn(clashing_extern_declarations)]` on by default
```

In a normal project there is one declaration and no warning. The fix is a
process, not a compiler feature: translate from the header, type by type, and
have the result checked (lesson 03).

**A struct without `repr(C)`.** The compiler can see this one:

```text
warning: `extern` block uses type `Pair`, which is not FFI-safe
 --> src/main.rs:9:47
  |
9 |     pub fn div(numer: c_int, denom: c_int) -> Pair;
  |                                               ^^^^ not FFI-safe
  |
  = help: consider adding a `#[repr(C)]` or `#[repr(transparent)]` attribute to this struct
  = note: this struct has unspecified layout
```

It is a warning, so the program still builds. Treat every `improper_ctypes`
warning as an error.

**Testing the return value instead of `errno`.** `strtol` returns `LONG_MAX`
for the text of the largest `long` and also for a number that is too large.
Only `errno == ERANGE` tells them apart, and only if `errno` was 0 before the
call.

**Calling C with an argument that C does not define.** Each C function has a
domain. Outside it, different C libraries do different things: one returns 0,
another ends the process. A wrapper checks the domain before the call and
returns its own error.

### One shape that the lesson leaves out: variadic functions

A C function such as `int printf(const char *format, ...)` takes a variable
number of arguments. Rust can declare and call one: the declaration in the
`unsafe extern "C"` block ends with `...` too. Two rules of C come with it. C
converts small arguments in the variable part before the call: `float`
becomes `double`, and integer types smaller than `int` become `int`. Rust
never converts silently, so it refuses the types that C would convert, and
tells you what to write:

```text
error[E0617]: can't pass `f32` to variadic function
 --> src/main.rs:9:39
  |
9 |     unsafe { printf(c"%f\n".as_ptr(), small) };
  |                                       ^^^^^
  |
help: cast the value to `c_double`
```

The second rule has no diagnostic: nothing compares the arguments with the
format string. `%d` with a `c_long` argument compiles in C and in Rust, and
is undefined behavior in both.

### Read more

- [`std::ffi`](https://doc.rust-lang.org/std/ffi/index.html): the C type aliases.
- [The Rust Reference: external blocks](https://doc.rust-lang.org/reference/items/external-blocks.html).
- [The Rustonomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html).
- [`pointer::offset_from_unsigned`](https://doc.rust-lang.org/std/primitive.pointer.html#method.offset_from_unsigned): its safety rules.

## Lesson 02 · Building C from Cargo: build.rs, the cc crate, `links`, and a `-sys` crate

**Assumed:** lesson 01; Chapter 8 (packages, features, dependencies).

### The idea

Until now the C code was already on the machine, inside the C library. To use
C code that you ship yourself, three things must happen before rustc compiles
your crate: a C compiler turns the `.c` files into object files, an archiver
packs them into a static library, and rustc is told to link that library.

Cargo's hook for this is the *build script*: a Rust program named `build.rs`
next to `Cargo.toml`. Cargo compiles it and runs it before it compiles the
package. The script and Cargo talk in two directions.

*Cargo to the script*, through environment variables:

| Variable | Meaning |
| --- | --- |
| `OUT_DIR` | The only directory the script may write to |
| `CARGO_MANIFEST_DIR` | The package directory (also the working directory) |
| `TARGET`, `HOST` | The platform being compiled for, and the one compiling |
| `OPT_LEVEL`, `DEBUG`, `PROFILE` | The build profile |
| `CARGO_FEATURE_<NAME>` | Set for each enabled feature of this package |
| `DEP_<LINKS>_<KEY>` | Metadata from the build script of a dependency (see below) |

*The script to Cargo*, through lines that it prints to standard output:

| Line | Effect |
| --- | --- |
| `cargo::rustc-link-lib=static=NAME` | Link `libNAME.a` (`NAME.lib` with MSVC) |
| `cargo::rustc-link-search=native=DIR` | Look for libraries in `DIR` |
| `cargo::rerun-if-changed=PATH` | Run the script again when this file changes |
| `cargo::rerun-if-env-changed=VAR` | Run it again when this variable changes |
| `cargo::rustc-env=VAR=VALUE` | Make `env!("VAR")` work in the crate |
| `cargo::rustc-cfg=NAME` | Enable `#[cfg(NAME)]` in the crate |
| `cargo::metadata=KEY=VALUE` | Pass a value to dependents (needs `links`) |

Older scripts and crates write `cargo:` with one colon, as the `cc` crate
does. Both spellings work, with one exception: the old spelling of the
metadata line is `cargo:KEY=VALUE`, without the word `metadata`.

The [`cc`](https://docs.rs/cc) crate does the compiler work. It finds the C
compiler (`cc`, or the one in the `CC` variable, or MSVC through the registry),
chooses flags that match `TARGET`, `OPT_LEVEL` and `DEBUG`, compiles, archives
into `OUT_DIR`, and prints the two link lines. It is a *build dependency*: it
is compiled for the host and linked into the build script only, never into
your crate.

### Worked example: this package's own build

Open `mastery/m6_ffi/build.rs` and read it with this section. The order of
events in one `cargo build`:

```text
1. Cargo compiles the build dependency `cc`          (for the host)
2. Cargo compiles build.rs and links it with `cc`    (for the host)
3. Cargo runs the build script:
       cc: kvtab.c         -> OUT_DIR/...kvtab.o        -> OUT_DIR/libkvtab.a
       cc: layout_probe.c  -> OUT_DIR/...layout_probe.o -> OUT_DIR/libm6_layout_probe.a
       the script prints its `cargo::` lines
4. Cargo compiles src/lib.rs with rustc              (for the target)
       -L native=OUT_DIR  -l static=kvtab  -l static=m6_layout_probe
```

This is what the script printed in a real run, without the 70
`rerun-if-env-changed` lines that the `cc` crate printed on that machine:

```text
cargo::rerun-if-changed=build.rs
cargo::rerun-if-changed=vendor/kvtab/kvtab.c
cargo::rerun-if-changed=vendor/kvtab/kvtab.h
cargo::rerun-if-changed=csrc/layout_probe.c
cargo:rustc-link-lib=static=kvtab
cargo:rustc-link-search=native=<target>/debug/build/m6_ffi-9452fbfa6ffcc64f/out
cargo:rustc-link-lib=static=m6_layout_probe
cargo:rustc-link-search=native=<target>/debug/build/m6_ffi-9452fbfa6ffcc64f/out
cargo::metadata=include=<repo>/mastery/m6_ffi/vendor/kvtab
cargo::rustc-env=M6_TARGET=aarch64-apple-darwin
cargo::rustc-env=M6_HOST=aarch64-apple-darwin
cargo::rustc-env=M6_LINKS=kvtab
cargo::rustc-env=M6_SCRATCH_DIR=<target>/m6-nested
```

You can see the output of any build script with `cargo build -vv`, or in the
file `output` next to the script's `OUT_DIR`.

### The `links` key and the `-sys` convention

`Cargo.toml` of this package contains `links = "kvtab"`. The key does two
things.

First, it is a claim: "this package links the native library kvtab." Cargo
allows one package per `links` value in a dependency graph. Two copies of a C
library in one program would define every symbol twice, and C has no modules
to keep them apart. Cargo refuses before anything is compiled:

```text
package `b-sys` links to the native library `fletch`, but it conflicts with a previous package which links to `fletch` as well:
package `a-sys v0.1.0 (/work/a-sys)`
    ... which satisfies path dependency `a-sys` of package `app v0.1.0 (/work/app)`
note: only one package in the dependency graph may specify the same links value to ensure that only one copy of a native library is linked in the final binary
```

Second, it opens a channel. A line `cargo::metadata=include=/some/dir` in the
build script of a package with `links = "kvtab"` becomes the environment
variable `DEP_KVTAB_INCLUDE` in the build scripts of packages that depend
**directly** on it. A dependent that compiles C code of its own against
`kvtab.h` finds the header this way. The value must be an absolute path: the
dependent's script runs in another directory.

By convention the package that owns the `links` value is named `NAME-sys` and
contains only two things: the build script that provides the library, and the
raw declarations. No safe wrappers, no conversions. The safe API is a second
crate that depends on the `-sys` crate. The split has a reason: there can be
several safe wrappers with different designs, but there can be only one
package that links the library, so they must all be able to share it.

A published `-sys` crate usually offers two ways to get the library: compile
the vendored sources (as here), or find a copy that is installed on the
system, for which the [`pkg-config`](https://docs.rs/pkg-config) crate is the
usual tool. Which one is the default is a decision with consequences: vendored
sources always build but must be updated by you when the library has a
security fix; a system copy is updated by the system but may be missing or too
old.

### Mistakes people make

**The C file changes and nothing is rebuilt.** A build script that prints no
`rerun-if` line is run again when any file of the package changes. As soon as
it prints one, Cargo watches only what the script names. The `cc` crate prints
`rerun-if-env-changed` lines for `CC`, `CFLAGS` and many more, and none for
your source files. So a script that calls `cc` and prints nothing itself is
never run again for a changed `.c` file. This was tried for this section: the
`rerun-if-changed` lines were removed from a finished copy of the lab's build
script, and then the C code was changed. The program printed the old
checksum, `c8f0`, before and after the change. With the lines restored it
printed the new one. Name every C input, including headers.

**A Cargo feature and a C macro that disagree.** If `lib.rs` declares a
function under `#[cfg(feature = "wide")]` but the build script does not define
the macro that makes the C compiler emit it, the declaration has nothing to
link to. This is the linker's message from such a build on macOS (other
linkers word it differently):

```text
Undefined symbols for architecture arm64:
  "_fletch32", referenced from:
      ... checker10checksum32 in checker-d44075c73f72c919...rcgu.o
ld: symbol(s) not found for architecture arm64
clang: error: linker command failed with exit code 1 (use -v to see invocation)
```

The feature must drive both sides: the script reads `CARGO_FEATURE_WIDE` and
defines the macro, and the declaration has the matching `cfg`.

**Writing outside `OUT_DIR`.** Generated files, object files and archives go
into `OUT_DIR`. A script that writes into the source tree breaks builds from a
read-only checkout and makes two builds with different features overwrite
each other.

**Doing too much.** A build script runs on every machine that builds your
crate, including ones without network access. Do not download anything in it.

### When to use which

- **`cc`** for a few C files with simple flags. This is the common case.
- **The `cmake` crate** (or similar) when the library has a build system of
  its own that you must not reimplement.
- **`pkg-config`** to use a library that is installed on the system.
- **Static linking** (as here) gives one self-contained program. **Dynamic
  linking** (`rustc-link-lib=dylib=NAME`) shares the library between programs
  and lets the system update it, but the library must be present at run time.

### Read more

- [The Cargo book: build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html).
- [The Cargo book: the `links` manifest key](https://doc.rust-lang.org/cargo/reference/build-scripts.html#the-links-manifest-key).
- [The Cargo book: `*-sys` packages](https://doc.rust-lang.org/cargo/reference/build-scripts.html#-sys-packages).
- [The Cargo book: environment variables for build scripts](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts).

## Lesson 03 · Bindings by hand: translate a C header and prove the translation

**Assumed:** lessons 01 and 02; Chapter 24, lessons 01, 03 and 05.

### The idea

*Bindings* are the Rust declarations of a C library: one Rust item for each
item of the header. Writing them is translation, and the rules are fixed.

| In the header | In Rust |
| --- | --- |
| `int`, `long`, `size_t`, `uint8_t`, ... | `c_int`, `c_long`, `usize`, `u8`, ... |
| `const T *` / `T *` | `*const T` / `*mut T` |
| `struct s { ... };` used by value or through pointers you read | `#[repr(C)] pub struct s { ... }`, same fields, same order |
| `typedef struct s s;` with no definition (opaque) | a `#[repr(C)]` struct with no public fields (below) |
| `enum e { A = 0, B = 1 };` | `pub const A: c_int = 0;` and so on |
| `int (*callback)(void *)` | `Option<unsafe extern "C" fn(*mut c_void) -> c_int>` |
| a function prototype | the same signature in an `unsafe extern "C"` block |

Three rows need a reason.

**Enums become constants.** A Rust `enum` may only ever hold one of its
variants; any other bit pattern is undefined behavior at the moment the value
exists. A C `enum` is an integer, and a newer version of the library may
return a value that your bindings do not list. An integer constant has no such
problem. There is a second reason: C does not fix the integer type of an enum.
`bindgen` (see below) reported `unsigned int` for the enums of this module's
library on macOS; with the MSVC compiler the same enums are `int`. Good C
interfaces therefore never use the enum type in a function signature, only
`int`.

**Function pointers become `Option`.** A C function pointer may be NULL. A
Rust `fn` pointer is never null, and a null one is undefined behavior as soon
as it exists. `Option<extern "C" fn(...)>` has the same size and the same ABI
as the C pointer, and `None` is NULL; the language guarantees this.

**Opaque structs.** For `typedef struct kvtab kvtab;` C code never sees the
fields. Rust needs a type to point to, and the type must not promise anything
false. The recommended form is:

```rust
use std::marker::{PhantomData, PhantomPinned};

#[repr(C)]
pub struct handle {
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

assert_eq!(size_of::<handle>(), 0);
```

The private fields stop other code from creating a value. The raw pointer in
the marker makes the type neither `Send` nor `Sync`, so that nothing is
assumed about threads. `PhantomPinned` makes it `!Unpin`, because a C object
must not be moved by Rust. The size is 0 because Rust does not know the real
size: you can hold a `*mut handle`, and you can never hold a `handle`.

### How `repr(C)` lays out a struct

The C compiler places each field at the next offset that is a multiple of the
field's alignment, and rounds the total size up to a multiple of the largest
alignment. `repr(C)` makes Rust do the same.

```text
struct span { uint32_t start; uint16_t len; uint8_t kind; };

offset   0    1    2    3    4    5    6    7
       +----+----+----+----+----+----+----+----+
       |       start       |   len   |kind| pad|
       +----+----+----+----+----+----+----+----+
        align 4             align 2   align 1
size 8 (7 rounded up to a multiple of 4), alignment 4
```

### Worked example: a header with a struct, a callback type and a function

The header:

```c
struct span {
    uint32_t start;
    uint16_t len;
    uint8_t kind;
};

typedef int (*span_filter)(const struct span *span, void *userdata);

/* Counts the spans for which `keep` returns non-zero; all, if `keep` is NULL. */
size_t span_count(const struct span *spans, size_t n, span_filter keep,
                  void *userdata);
```

The translation. So that the example runs by itself, the function is defined
here in Rust with the C ABI; in a real binding it would be a declaration in an
`unsafe extern "C"` block, with the body in a C library.

```rust
#![allow(non_camel_case_types)]
use std::ffi::{c_int, c_void};
use std::mem::offset_of;

/// `struct span`
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct span {
    pub start: u32,
    pub len: u16,
    pub kind: u8,
}

/// `span_filter`
pub type span_filter =
    Option<unsafe extern "C" fn(span: *const span, userdata: *mut c_void) -> c_int>;

/// `size_t span_count(const struct span *, size_t, span_filter, void *);`
///
/// # Safety
/// `spans` must point to `n` spans, and `keep` must accept `userdata`.
pub unsafe extern "C" fn span_count(
    spans: *const span,
    n: usize,
    keep: span_filter,
    userdata: *mut c_void,
) -> usize {
    let Some(keep) = keep else { return n };
    (0..n)
        // SAFETY: the caller promises `n` spans and a matching `userdata`.
        .filter(|&index| unsafe { keep(spans.add(index), userdata) } != 0)
        .count()
}

/// A filter: keeps spans longer than the `u16` that `userdata` points to.
unsafe extern "C" fn longer_than(span: *const span, userdata: *mut c_void) -> c_int {
    // SAFETY: span_count passes a valid span; `main` passes a pointer to a u16.
    let (span, limit) = unsafe { (&*span, *userdata.cast::<u16>()) };
    c_int::from(span.len > limit)
}

fn main() {
    // The layout that the diagram above predicts.
    assert_eq!((size_of::<span>(), align_of::<span>()), (8, 4));
    assert_eq!(offset_of!(span, start), 0);
    assert_eq!(offset_of!(span, len), 4);
    assert_eq!(offset_of!(span, kind), 6);

    let spans = [
        span { start: 0, len: 5, kind: 1 },
        span { start: 5, len: 1, kind: 2 },
        span { start: 6, len: 9, kind: 1 },
    ];
    let mut limit: u16 = 4;
    // SAFETY: the pointer and the length describe `spans`; `longer_than`
    // reads `userdata` as the u16 that it is.
    let long = unsafe {
        span_count(spans.as_ptr(), spans.len(), Some(longer_than), (&raw mut limit).cast())
    };
    assert_eq!(long, 2);
    // SAFETY: as above; a NULL filter is allowed and ignores `userdata`.
    let all = unsafe { span_count(spans.as_ptr(), spans.len(), None, std::ptr::null_mut()) };
    assert_eq!(all, 3);
}
```

### Who checks the translation?

The Rust compiler cannot: it never reads the header. If `len` were declared
`u32`, everything would compile, and `kind` would be read from the wrong byte.
There are four ways to get a check, and this lesson uses the first three.

1. **Ask the C compiler.** Compile a small C file, with the same compiler and
   flags as the library, that returns `sizeof`, `offsetof` and the values of
   the enumerators. Compare them with `size_of`, `offset_of!` and your
   constants in a test. `csrc/layout_probe.c` is such a file.
2. **Compare with a second translation.** When one crate declares the same
   symbol twice, the `clashing_extern_declarations` lint compares the two
   signatures, including the fields of `repr(C)` structs behind pointers.
3. **Run under Miri against a model.** Miri checks each call against the
   definition that it resolves to. In this module that definition is a Rust
   model of the library.
4. **Generate the bindings.** The [`bindgen`](https://rust-lang.github.io/rust-bindgen/)
   tool reads the header with a real C parser (libclang) and writes the Rust
   declarations. For the `span` header above, bindgen 0.73.2 printed:

```text
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct span {
    pub start: u32,
    pub len: u16,
    pub kind: u8,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of span"][::std::mem::size_of::<span>() - 8usize];
    ["Alignment of span"][::std::mem::align_of::<span>() - 4usize];
    ["Offset of field: span::start"][::std::mem::offset_of!(span, start) - 0usize];
    ["Offset of field: span::len"][::std::mem::offset_of!(span, len) - 4usize];
    ["Offset of field: span::kind"][::std::mem::offset_of!(span, kind) - 6usize];
};
pub type span_filter = ::std::option::Option<
    unsafe extern "C" fn(
        span: *const span,
        userdata: *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int,
>;
unsafe extern "C" {
    pub fn span_count(
        spans: *const span,
        n: usize,
        keep: span_filter,
        userdata: *mut ::std::os::raw::c_void,
    ) -> usize;
}
```

The `const _` block is a layout test: each line indexes a one-element array
with `actual - expected`, which compiles only when the difference is 0.

### When to use which

- **Write by hand** when the header is small and stable, as here. You get
  readable declarations and no extra build requirement, and you must provide
  the check yourself.
- **Run bindgen in the build script** when the header is large or differs
  between versions and platforms. Every user then needs libclang installed.
- **Run bindgen once and commit the output** to avoid that requirement.
  Generated bindings describe the one target they were generated for, so
  this fits headers that use only fixed-size types, or needs one file per
  target.

In every case the `-sys` layer stays a mirror of the header. Decisions about
safety belong to the wrapper (lesson 04).

### What a header can contain that this lesson does not

`kvtab.h` was written to be easy to bind. Other headers have items for which
the table at the start of this section has no row:

- **Macros and `static inline` functions.** A `#define` and an inline
  function have no symbol in the compiled library, so there is nothing to
  link to. A constant must be repeated in Rust, as you do for the
  enumerators. An inline function or a function-like macro must be written
  again in Rust, or wrapped in a small C file of your own that the build
  script compiles.
- **Unions.** `union u { int i; float f; };` becomes `#[repr(C)] union`.
  Reading a field of a union is `unsafe` in Rust (Chapter 23, lesson 05).
- **Bit-fields.** `unsigned flag : 1;` has no Rust syntax, and its layout is
  decided by the C compiler. Use bindgen for such a struct, which generates
  accessor functions, or keep the struct opaque and add C functions that
  read the fields.
- **Packed structs.** `#pragma pack(1)` corresponds to `#[repr(C, packed)]`.
  A field of such a struct may be at an unaligned address, and the compiler
  refuses a reference to it (Chapter 23, lesson 09).
- **C++.** Rust cannot call C++ functions directly. A C++ library needs
  wrapper functions that are declared `extern "C"` on the C++ side.

### Mistakes people make

- **`bool` or a Rust `enum` for a C integer.** C may store 2 in an
  `unsigned char` flag. A Rust `bool` with the value 2 is undefined behavior.
  Translate the type that the header says, and convert with `!= 0` in the
  wrapper.
- **`i64` for `long`**, which is wrong on Windows, and **`u32` for `size_t`**,
  which is wrong on every 64-bit target.
- **Dropping a `const`.** `const T *` is `*const T`. Writing `*mut T` makes
  later code believe that the function may write.
- **Reading a struct that C filled before checking the status code.** The
  fields are initialised only if the function says that it succeeded. Use
  `MaybeUninit<T>`, pass `as_mut_ptr()`, and call `assume_init()` after the
  status check.

### Read more

- [The Rust Reference: type layout, the `C` representation](https://doc.rust-lang.org/reference/type-layout.html#the-c-representation).
- [The Rustonomicon: representing opaque structs](https://doc.rust-lang.org/nomicon/ffi.html#representing-opaque-structs).
- [`std::mem::MaybeUninit`: out-pointers](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html#out-pointers).
- [The `clashing_extern_declarations` lint](https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#clashing-extern-declarations).

## Lesson 04 · A safe wrapper: ownership, error mapping, Drop, and threads

**Assumed:** lesson 03; Chapter 13 (`Drop`); Chapter 19 (`Send`, `Sync`).

### The idea

Raw bindings are unsafe to call because each function has preconditions that
the compiler cannot see: "the handle is live", "no other call is in progress",
"the key is NUL-terminated". A *safe wrapper* is a Rust type whose public API
makes those preconditions impossible to break. It works in four steps.

**1. Write the invariant.** Decide what is always true about a value of your
type, for example: "`raw` points to a live table that this value alone owns."
Keep the field private. Every constructor must establish the invariant, every
method must keep it, and every `unsafe` block is justified by pointing at it.

**2. Tie the C lifetime to Rust ownership.** The constructor calls the
library's "new" function, and `Drop` calls its "free" function. Then "freed
exactly once" and "never used after free" follow from Rust's ownership rules,
provided that the type is neither `Clone` nor `Copy`.

**3. Choose `&self` or `&mut self` for each method.** The compiler does not
need `&mut self` to call a C function through a raw pointer. You choose it to
express a rule of the library: a function that changes the object, or that
ends the validity of pointers handed out earlier, takes `&mut self`. Then the
borrow checker enforces "valid until the next change" for you (lesson 06
depends on this).

**4. Map errors without losing information.** A status code is a C `int`. The
conversion to a Rust error type needs a place for every `int`, also for the
ones that today's header does not list.

One more decision belongs to the wrapper: threads. A struct that contains a
raw pointer is neither `Send` nor `Sync`, because the compiler knows nothing
about the object behind the pointer. You may add `unsafe impl Send` or `Sync`,
and the only acceptable evidence is the library's documentation.

| The documentation says | The wrapper may be |
| --- | --- |
| Objects are tied to the thread that created them | neither |
| No global state; calls on one object must not overlap | `Send` |
| Reading functions may also run at the same time | `Send` and `Sync`, if every `&self` method is a reading function |
| Nothing | neither |

Your own reading of today's source code is not evidence: the next version may
add a cache inside a "reading" function.

### Worked example: a gauge

The "C library" here is a gauge with a limit. It is written in Rust with the C
ABI, so that the example runs by itself, and it counts its live objects so
that the example can show the effect of `Drop`.

```rust
use std::ffi::c_int;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---- The library: gauge_new, gauge_free, gauge_add, gauge_level ----
pub struct RawGauge {
    level: u32,
    limit: u32,
}
static LIVE: AtomicUsize = AtomicUsize::new(0);
const GAUGE_OK: c_int = 0;
const GAUGE_ERR_LIMIT: c_int = 1;

extern "C" fn gauge_new(limit: u32) -> *mut RawGauge {
    LIVE.fetch_add(1, Ordering::SeqCst);
    Box::into_raw(Box::new(RawGauge { level: 0, limit }))
}
/// # Safety: `gauge` must come from `gauge_new` and not be used again.
unsafe extern "C" fn gauge_free(gauge: *mut RawGauge) {
    LIVE.fetch_sub(1, Ordering::SeqCst);
    // SAFETY: promised by the caller.
    drop(unsafe { Box::from_raw(gauge) });
}
/// # Safety: `gauge` must be live, with no other call in progress.
unsafe extern "C" fn gauge_add(gauge: *mut RawGauge, amount: u32) -> c_int {
    // SAFETY: promised by the caller.
    let gauge = unsafe { &mut *gauge };
    match gauge.level.checked_add(amount) {
        Some(level) if level <= gauge.limit => {
            gauge.level = level;
            GAUGE_OK
        }
        _ => GAUGE_ERR_LIMIT,
    }
}
/// # Safety: `gauge` must be live.
unsafe extern "C" fn gauge_level(gauge: *const RawGauge) -> u32 {
    // SAFETY: promised by the caller.
    unsafe { (*gauge).level }
}

// ---- The safe wrapper ----
#[derive(Debug, PartialEq, Eq)]
pub enum GaugeError {
    OverLimit,
    /// A status that this wrapper does not know. Never thrown away.
    Unexpected(c_int),
}

pub struct Gauge {
    /// Invariant: a live gauge from `gauge_new`, owned by this value alone.
    raw: NonNull<RawGauge>,
}

impl Gauge {
    pub fn new(limit: u32) -> Gauge {
        let raw = NonNull::new(gauge_new(limit)).expect("gauge_new never returns NULL");
        Gauge { raw }
    }

    /// `&mut self`: the call changes the gauge.
    pub fn add(&mut self, amount: u32) -> Result<(), GaugeError> {
        // SAFETY: `raw` is live (invariant), and `&mut self` means that no
        // other call on this gauge is in progress.
        match unsafe { gauge_add(self.raw.as_ptr(), amount) } {
            GAUGE_OK => Ok(()),
            GAUGE_ERR_LIMIT => Err(GaugeError::OverLimit),
            other => Err(GaugeError::Unexpected(other)),
        }
    }

    pub fn level(&self) -> u32 {
        // SAFETY: `raw` is live (invariant).
        unsafe { gauge_level(self.raw.as_ptr()) }
    }
}

impl Drop for Gauge {
    fn drop(&mut self) {
        // SAFETY: `raw` is live and owned by this value, and `drop` runs once.
        unsafe { gauge_free(self.raw.as_ptr()) };
    }
}

fn main() {
    {
        let mut gauge = Gauge::new(10);
        assert_eq!(gauge.add(7), Ok(()));
        assert_eq!(gauge.add(7), Err(GaugeError::OverLimit));
        assert_eq!(gauge.level(), 7);
        assert_eq!(LIVE.load(Ordering::SeqCst), 1);
    }
    assert_eq!(LIVE.load(Ordering::SeqCst), 0, "Drop freed the gauge");
    // `NonNull` leaves the NULL pattern free, so `Option<Gauge>` costs nothing.
    assert_eq!(size_of::<Option<Gauge>>(), size_of::<*mut RawGauge>());
}
```

```text
stack                      the library's heap
+----------------+         +------------------+
| Gauge          |         | RawGauge         |
|   raw ---------+-------> |   level: 7       |
+----------------+         |   limit: 10      |
 one owner                 +------------------+
 Drop -> gauge_free         freed exactly once
```

This `Gauge` has no `unsafe impl Send`: the example library has no
documentation to justify one. In the lesson you have a header to read.

### Handing a handle out, and taking one in

A complete wrapper lets its user leave the safe API, the way `Box` does with
`into_raw` and `from_raw`. `as_raw(&self)` lends the pointer. `into_raw(self)`
gives ownership away, so it must return the pointer **without** running
`Drop`; `ManuallyDrop` or `mem::forget` does that. `from_raw` is an
`unsafe fn`, because only the caller can know that the pointer is live and
has no other owner. Its `# Safety` section repeats the invariant of the type,
word for word, as a duty of the caller.

### Mistakes people make

**`#[derive(Clone)]` on the wrapper.** It compiles, copies the pointer, and
`Drop` then frees the same object twice. There is no diagnostic. A handle is
`Clone` only if the library has a function that makes a real copy or
increments a reference count.

**A `match` on the status without a catch-all arm.** The compiler refuses it
(this is drill 02, whose constants are 0, 2 and 3):

```text
error[E0004]: non-exhaustive patterns: `i32::MIN..=-1_i32`, `1_i32` and `4_i32..=i32::MAX` not covered
  --> src/main.rs:18:11
   |
18 |     match status {
   |           ^^^^^^ patterns `i32::MIN..=-1_i32`, `1_i32` and `4_i32..=i32::MAX` not covered
...
   = note: the matched value is of type `i32`
```

Do not answer this by converting the integer into a Rust `enum` with
`transmute`: that moves the missing case to a place where it is undefined
behavior instead of a compile error. Add a variant that keeps the number.

**Using the wrapper on a second thread without thinking.** The compiler stops
you, and the message names the raw pointer (this is drill 04):

```text
error[E0277]: `*mut u32` cannot be sent between threads safely
   --> src/main.rs:40:30
    |
 40 |     let seen = thread::spawn(move || handle.get()).join().unwrap(); // KEEP
    |                ------------- -------^^^^^^^^^^^^^
    |                |             |
    |                |             `*mut u32` cannot be sent between threads safely
...
    = help: within `{closure@src/main.rs:40:30: 40:37}`, the trait `Send` is not implemented for `*mut u32`
note: required because it appears within the type `Handle`
```

The fix is not "add `unsafe impl Send` until it compiles". It is: read what
the library says about threads, then write the impl with that sentence as its
SAFETY comment, or do not write it.

**`&self` on a method that changes the object.** It compiles and seems
convenient. It becomes unsound the day another method hands out a borrowed
view of the object's memory.

### Read more

- [The Rustonomicon: `Send` and `Sync`](https://doc.rust-lang.org/nomicon/send-and-sync.html).
- [`std::ptr::NonNull`](https://doc.rust-lang.org/std/ptr/struct.NonNull.html).
- [`std::mem::ManuallyDrop`](https://doc.rust-lang.org/std/mem/struct.ManuallyDrop.html).
- [`Box::into_raw`](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.into_raw) and [`Box::from_raw`](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.from_raw): the model for handing a handle out and taking it back.

## Lesson 05 · Callbacks: userdata, trampolines, closures that C owns, and panics at the boundary

**Assumed:** lesson 04; Chapter 24, lesson 04 (callbacks and userdata);
Chapter 14 (closures and the `Fn` traits).

### The idea

A C callback is a function pointer: one address, nothing else. A Rust closure
is code **and** captured data. To give a closure to C you take the two apart:

- the **data** travels as the `void *userdata` argument that every well
  designed C callback API has, and that the library passes back unchanged;
- the **code** is an `extern "C"` function that you write, often called a
  *trampoline*. It receives `userdata`, casts it back to the closure's type,
  and calls the closure.

The trampoline must know the closure's type to cast `userdata` back. Make it
generic: `unsafe extern "C" fn trampoline<F: FnMut(...)>(..., userdata: *mut c_void)`.
The compiler creates one copy for each closure type, every copy has the same C
signature, and inside the copy `userdata.cast::<F>()` is the right cast.
`Some(trampoline::<F>)` is then the function pointer for C.

Why not pass `&mut dyn FnMut(...)`? A pointer to a `dyn` type is two words (the
data pointer and the vtable pointer), and `void *` is one. The generic
trampoline needs no vtable and no allocation.

There are two kinds of callback, and they differ in who owns the closure.

```text
called during one C call                   kept by C for later
(for example a "for each" function)        (for example an observer or a timer)

closure lives on the caller's stack        closure lives in a Box
userdata = pointer to it                   userdata = Box::into_raw(box)
valid until the C call returns             valid until C calls your "drop" function
nothing to free                            the drop function does Box::from_raw
no 'static bound needed                    needs F: 'static, and F: Send if the
                                           owner can move to another thread
```

For the second kind the library must offer a way to release `userdata`: a
"destroy" or "drop" function pointer that it calls exactly once when it lets
go of the callback. If a library keeps your pointer and has no such hook, you
must keep the closure alive yourself for as long as the library might call it,
and release it afterwards.

### Worked example: a callback for the length of one call

`each_digit` stands in for a C function; it is written in Rust with the C ABI
so that the example runs by itself.

```rust
use std::ffi::{c_int, c_void};

/// `void each_digit(unsigned n, int (*visit)(unsigned digit, void *), void *userdata);`
/// Calls `visit` for each decimal digit, last digit first, until it returns non-zero.
///
/// # Safety
/// `visit` must accept `userdata`.
unsafe extern "C" fn each_digit(
    mut n: u32,
    visit: unsafe extern "C" fn(digit: u32, userdata: *mut c_void) -> c_int,
    userdata: *mut c_void,
) {
    loop {
        // SAFETY: the caller promises that `visit` accepts `userdata`.
        if unsafe { visit(n % 10, userdata) } != 0 || n < 10 {
            return;
        }
        n /= 10;
    }
}

/// One copy of this function exists for each closure type `F`.
unsafe extern "C" fn trampoline<F: FnMut(u32) -> bool>(digit: u32, userdata: *mut c_void) -> c_int {
    // SAFETY: `digits` below passes a pointer to its own `F`, which lives
    // until `each_digit` returns, and does not use it while C runs.
    let f = unsafe { &mut *userdata.cast::<F>() };
    c_int::from(f(digit))
}

/// The safe API: calls `f` with each digit; `f` returns `true` to stop.
fn digits<F: FnMut(u32) -> bool>(n: u32, mut f: F) {
    // SAFETY: `trampoline::<F>` reads `userdata` as an `F`, and that is what
    // it gets. `f` outlives the call, and each_digit keeps no pointer.
    unsafe { each_digit(n, trampoline::<F>, (&raw mut f).cast::<c_void>()) };
}

fn main() {
    let mut seen = Vec::new();
    digits(4071, |digit| {
        seen.push(digit);
        false
    });
    assert_eq!(seen, [1, 7, 0, 4]);

    let mut sum = 0;
    digits(4071, |digit| {
        sum += digit;
        digit == 0 // stop at the first zero
    });
    assert_eq!(sum, 8);
}
```

```text
the stack while the closure runs (it grows downwards):

  main                        Rust
  digits::<F>                 Rust    owns `f`; userdata points here
  each_digit                  "C"     knows only two addresses
  trampoline::<F>             Rust    casts userdata back to &mut F
  the closure                 Rust
```

This example is incomplete in one way, and the lesson asks you to complete
it: what if the closure panics?

### Panics at the boundary

A panic unwinds the stack: it runs destructors frame by frame until something
catches it. In the picture above, the frame below `trampoline` belongs to C.
C code has no destructors and may have no unwinding tables, and a library that
is interrupted in the middle of a function may be left in a broken state. So
the rule of the `"C"` ABI is: **a function with this ABI does not unwind.**
Rust enforces its side of the rule. When a panic tries to leave an
`extern "C"` function, the process is aborted. This is what the abort looks
like (drill 08):

```text
thread 'main' (125450958) panicked at src/main.rs:17:9:
7 is odd
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'main' (125450958) panicked at /rustc/b940084d7eb6a299eb4bfeb8e34901bc051e7ac4/library/core/src/panicking.rs:225:5:
panic in a function that cannot unwind
...
thread caused non-unwinding panic. aborting.
```

The abort is a safety net, not a design. A wrapper has three designs to choose
from.

- **Catch, stop, continue later.** In the trampoline, wrap the call of the
  closure in `std::panic::catch_unwind`. If it returns `Err(payload)`, store
  the payload next to the closure (so `userdata` points to a struct that
  holds both), and return the value that tells the library to stop. When the
  C function has returned and its frames are gone, the wrapper checks for a
  stored payload and calls `std::panic::resume_unwind(payload)`. For the
  caller, the panic looks as if it had passed straight through. This is right
  for callbacks that run during one call.
- **Abort on purpose.** For a callback that the library calls later, from
  some other call, there may be nobody to hand the panic to. Leaving the
  `extern "C"` abort in place is then a legitimate choice, if the
  documentation says so.
- **`extern "C-unwind"`.** This ABI permits unwinding across the boundary. It
  is correct only when the foreign code was compiled to support being unwound
  through and has no state to clean up. For an ordinary C library, do not
  assume that.

`catch_unwind` wants its closure to be `UnwindSafe`, which a closure holding
`&mut` state is not. `AssertUnwindSafe` is the way to say "I have thought
about what state a caught panic leaves behind". Write the thought down next
to it.

### Releasing a closure that C owns

The drop function for a boxed closure is short. It is generic for the same
reason as the trampoline:

```rust
use std::ffi::c_void;
use std::rc::Rc;

/// What a library calls to release the userdata of a callback.
///
/// # Safety
/// `userdata` must come from `Box::into_raw(Box::<T>::new(..))`, and this
/// must be its only release.
unsafe extern "C" fn drop_box<T>(userdata: *mut c_void) {
    // SAFETY: promised by the caller.
    drop(unsafe { Box::from_raw(userdata.cast::<T>()) });
}

let shared = Rc::new(5);
let captured = Rc::clone(&shared);
let closure = move || *captured;
// The type of a closure has no name, so let inference carry it.
fn into_userdata<F: Fn() -> i32>(f: F) -> (*mut c_void, unsafe extern "C" fn(*mut c_void)) {
    (Box::into_raw(Box::new(f)).cast(), drop_box::<F>)
}
let (userdata, release) = into_userdata(closure);
assert_eq!(Rc::strong_count(&shared), 2); // the boxed closure holds one
// SAFETY: `userdata` came from Box::into_raw for the same type, once.
unsafe { release(userdata) };
assert_eq!(Rc::strong_count(&shared), 1); // the closure was dropped
```

### Mistakes people make

**Passing a closure where C wants a function pointer.** Only the data-free
part of a closure can be a function pointer (this is drill 01):

```text
error[E0308]: mismatched types
  --> src/main.rs:23:30
   |
23 |     let result = apply_twice(|n| n + step, 1);
   |                  ----------- ^^^^^^^^^^^^ expected fn pointer, found closure
   |                  |
   |                  arguments to this function are incorrect
   |
   = note: expected fn pointer `extern "C" fn(i32) -> i32`
                 found closure `{closure@src/main.rs:23:30: 23:33}`
note: closures can only be coerced to `fn` types if they do not capture any variables
```

**Userdata that dies before the last callback.** A pointer to a closure on the
stack is right for a callback that runs during one call, and a dangling
pointer for one that C keeps. The bounds `F: 'static` and "boxed, released by
the drop function" belong together.

**Two references to the same userdata.** The trampoline creates `&mut F` from
`userdata`. That is sound only if no other reference to the closure is in use
at that moment: the wrapper must not touch the closure while C may call it,
and the library must not call the callback again from inside itself. Write
both facts into the SAFETY comment; they are what a reviewer looks for.

**Forgetting `Send`.** If the wrapper type is `Send`, a closure that it owns
moves to the other thread with it and is called there. The compiler cannot
see through `void *`, so the bound `F: Send` on the registering method is the
only thing that keeps the `unsafe impl Send` true. The same bound is needed,
for a different reason, when the library calls the callback from a thread
that it started itself (a timer or a worker thread): then the closure runs
on a thread that Rust never saw. If such calls can overlap, the closure must
be `Fn` and `Sync` as well. kvtab has no threads of its own; read the
documentation of each library for this.

### Read more

- [`std::panic::catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html) and [`resume_unwind`](https://doc.rust-lang.org/std/panic/fn.resume_unwind.html).
- [The Rust Reference: unwinding across FFI boundaries](https://doc.rust-lang.org/reference/panic.html#unwinding-across-ffi-boundaries).
- [The Rustonomicon: FFI and unwinding](https://doc.rust-lang.org/nomicon/ffi.html#ffi-and-unwinding).
- [`std::panic::AssertUnwindSafe`](https://doc.rust-lang.org/std/panic/struct.AssertUnwindSafe.html).

## Lesson 06 · Strings and buffers across the boundary

**Assumed:** lesson 04; Chapter 24, lesson 02 (`CStr` and `CString`);
Chapter 17 (strings and UTF-8).

### The idea

For every pointer that crosses the boundary, in either direction, three
questions have exact answers, and the answers come from the library's
documentation:

1. **Who allocated the memory?**
2. **Who frees it, and with which function?**
3. **Until when may the other side use the pointer?**

If you cannot answer all three, you cannot write the SAFETY comment. The
answers fall into a small number of patterns.

| Pattern | Who frees | Valid until | In Rust |
| --- | --- | --- | --- |
| Rust lends to C for one call | Rust | the call returns | `as_ptr()` of a value that outlives the call |
| C returns a static string | nobody | the program ends | `&'static CStr` |
| C returns a pointer into its object | C, later | the object changes or is freed | a borrow tied to `&self` |
| C fills a buffer that Rust provides | Rust | as long as Rust wants | `&mut [u8]`, or a `Vec` filled through its spare capacity |
| C returns memory that the caller must free | Rust, with the **library's** free function | you free it | a wrapper type whose `Drop` calls that function |

Never free C's memory with Rust's allocator or Rust's memory with C's `free`.
They may be different allocators, and each one keeps its own bookkeeping next
to the block.

Strings add a second topic: which string type? No two of these are the same
thing.

| Type | Contents | Terminator | Length known |
| --- | --- | --- | --- |
| `&str`, `String` | valid UTF-8 | none | yes |
| `&CStr`, `CString` | any bytes except 0 | one 0 byte | found by scanning |
| `&[u8]`, `Vec<u8>` | any bytes | none | yes |
| `&OsStr`, `OsString`, `Path` | what the operating system uses for names | none | yes |
| `&[u16]` ending in 0 | UTF-16, not necessarily valid | one 0 unit | found by scanning |

Each conversion can fail or lose something, and the wrapper must decide what
happens:

- `&str` to `CString`: fails if the text contains a 0 byte, because C would
  stop reading there. `CString::new` reports the position.
- `&CStr` to `&str`: fails if the bytes are not UTF-8. `to_str()` returns the
  error; `to_string_lossy()` replaces bad bytes with U+FFFD; `to_bytes()`
  keeps everything.
- The last row is the form of the Windows API. A Rust string becomes one with
  `encode_utf16()` and a 0 at the end. The other direction must handle
  unpaired surrogates: `String::from_utf16_lossy`, or `OsString` on Windows.

### Worked example: a function that fills the caller's buffer

Many C functions follow the model of `snprintf`: the caller passes a buffer
and its size, and the function returns the size that the full result needs.
If the buffer was too small, the caller allocates and calls again.
`unit_name` stands in for such a function.

```rust
use std::ffi::{CStr, c_char, c_int};

/// `int unit_name(int code, char *buf, size_t len);`
/// Writes the name and a NUL if `len` is large enough; writes nothing
/// otherwise. Returns the length of the name without the NUL, or -1 for an
/// unknown code. `buf` may be NULL when `len` is 0.
///
/// # Safety
/// `buf` must be valid for writes of `len` bytes.
unsafe extern "C" fn unit_name(code: c_int, buf: *mut c_char, len: usize) -> c_int {
    let name: &CStr = match code {
        1 => c"metre",
        2 => c"kilogram",
        3 => c"second",
        _ => return -1,
    };
    let bytes = name.to_bytes_with_nul();
    if bytes.len() <= len {
        // SAFETY: the caller promises `len >= bytes.len()` writable bytes.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.cast::<u8>(), bytes.len()) };
    }
    (bytes.len() - 1) as c_int
}

/// The safe API.
fn unit(code: c_int) -> Option<String> {
    // First call: no buffer at all. The result is the length without the NUL.
    // SAFETY: with `len == 0` nothing is written, so NULL is allowed.
    let length = unsafe { unit_name(code, std::ptr::null_mut(), 0) };
    let length = usize::try_from(length).ok()?; // negative: unknown code

    // One allocation, not filled with zeros: `with_capacity` gives memory
    // that is owned but not initialised.
    let mut buf: Vec<u8> = Vec::with_capacity(length + 1);
    // SAFETY: `as_mut_ptr` is valid for writes of `capacity()` bytes, which
    // is the size we pass. Only a raw pointer to the uninitialised bytes
    // exists; no `&mut [u8]` to them is created.
    let written = unsafe { unit_name(code, buf.as_mut_ptr().cast(), buf.capacity()) };
    // Check what foreign code reports before an unsafe operation relies on it.
    assert_eq!(usize::try_from(written), Ok(length), "the name changed between two calls");
    // SAFETY: the function wrote `length + 1` bytes (the name and the NUL).
    // The first `length` are initialised and within the capacity.
    unsafe { buf.set_len(length) };
    String::from_utf8(buf).ok()
}

fn main() {
    assert_eq!(unit(2).as_deref(), Some("kilogram"));
    assert_eq!(unit(9), None);
}
```

```text
after the second call, for code 2:

  buf (Vec<u8>)    ptr ----> +---+---+---+---+---+---+---+---+----+
                   len 0     | k | i | l | o | g | r | a | m | \0 |
                   cap 9     +---+---+---+---+---+---+---+---+----+
                             written by C; the Vec does not know yet

  set_len(8):      len 8     the NUL stays in the spare capacity
```

The order of the two unsafe steps matters. `set_len` before the call would
break the documented rule of `set_len` (the elements up to the new length
must be initialised), and every `&buf[..]` afterwards would be a `&[u8]` to
bytes that are not initialised. The Rust Reference counts a reference to an
invalid value as undefined behavior even if nobody "really" reads the bytes.
It also says that this last rule is not settled yet, so write code that
does not depend on how it is settled: write first, then `set_len`.

### A pointer into the library's object

The third pattern in the table is the dangerous one, because the pointer stops
being valid without any call that mentions it. A typical sentence in a header
is: "The pointer is valid until the next call that changes the object." A raw
pointer cannot carry that sentence. A Rust signature can:

```text
fn view(&self) -> Option<&[u8]>     // the result borrows the wrapper
fn change(&mut self, ...)           // needs the wrapper exclusively
```

With lifetime elision, the returned slice borrows `self`. As long as the slice
is alive, the borrow checker refuses every `&mut self` method, and also
dropping or moving the wrapper. This is why lesson 04 insisted on `&mut self`
for the changing methods. The compiler's message when a caller tries (this is
drill 05):

```text
error[E0502]: cannot borrow `table` as mutable because it is also borrowed as immutable
  --> src/main.rs:32:5
   |
31 |     let view = table.peek(0);
   |                ----- immutable borrow occurs here
32 |     table.put(0, b"new");
   |     ^^^^^^^^^^^^^^^^^^^^ mutable borrow occurs here
33 |     assert_eq!(view, b"old"); // KEEP
   |     ------------------------ immutable borrow later used here
```

The SAFETY comment of such a method lists every way in which the library ends
the validity of the pointer, and for each one the reason why a caller with the
slice in hand cannot reach it.

### Mistakes people make

**A pointer to a temporary.** `as_ptr()` does not keep its `CString` alive.
The compiler has a lint for the most common form:

```text
warning: this creates a dangling pointer because temporary `CString` is dropped at end of statement
 --> src/main.rs:8:66
  |
8 |     let text: *const c_char = CString::new("temporary").unwrap().as_ptr();
  |                               ---------------------------------- ^^^^^^ pointer created here
  |                               |
  |                               this `CString` is dropped at end of statement
  |
  = help: bind the `CString` to a variable such that it outlives the pointer returned by `as_ptr`
```

It is a warning, the program compiles, and the call then reads freed memory.

**`text.as_ptr()` of a `&str` passed as a C string.** The types match after a
cast, and there is no NUL at the end. C reads on into whatever follows.

**Trusting a length from foreign code.** A length that C reports goes into
`set_len` or `from_raw_parts`, which are unsafe. Compare it with what you know
(the capacity, the first call's answer) before you use it.

**A wrong lifetime.** `CStr::from_ptr` returns a `&CStr` with a lifetime that
the compiler takes from the context, because a raw pointer has none. If the
context says `'static` and the truth is "until the next call", the compiler
is satisfied and the program is wrong. Give the function an explicit
signature that ties the result to its owner.

### When to use which

- **Copy** (`get` into a `Vec`) when the caller keeps the data, or when the
  library's validity rule is hard to express. **Borrow** (`peek`) when the
  data is large and used at once.
- **`to_str`** when invalid UTF-8 is an error that the caller should see,
  **`to_string_lossy`** for text that is only shown to people, **bytes** when
  the data must survive a round trip unchanged (file names, keys).

### Read more

- [`std::ffi::CStr`](https://doc.rust-lang.org/std/ffi/struct.CStr.html) and [`CString`](https://doc.rust-lang.org/std/ffi/struct.CString.html).
- [`Vec::spare_capacity_mut`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.spare_capacity_mut) and [`Vec::set_len`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.set_len).
- [`std::ffi::OsString`](https://doc.rust-lang.org/std/ffi/struct.OsString.html): platform strings and their conversions.
- [The `dangling_pointers_from_temporaries` lint](https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#dangling-pointers-from-temporaries).

## Lesson 07 · Exporting a Rust library to C: `extern "C"` definitions, a static library, a header

**Assumed:** lessons 03 to 06.

### The idea

So far Rust called C. In the other direction, Rust *defines* functions with
the C ABI and a C program calls them. Four things make a Rust function
callable from C.

| Part | What it does |
| --- | --- |
| `extern "C"` on the definition | The function uses the C calling convention |
| `#[unsafe(no_mangle)]` | The symbol gets exactly the function's name, not a mangled Rust name, and the library exports it |
| Only FFI-safe types in the signature | C can describe the types |
| A crate type that a C linker understands | See below |

`pub` is not in the table. It decides who may call the function *from Rust*
and whether the documentation shows it. The export comes from `no_mangle`
alone: a private function with that attribute, in a private module, is
still a symbol that any C program can link to. This was checked for static
and shared libraries with the course toolchain. It is also why the
attribute must be written `unsafe(...)`: the name enters the one flat
namespace of the linker, where it can collide with any other symbol of the
program. Exported functions are usually `pub` as well, so that Rust tests
and Rust users can reach them by path.

The **crate type** decides what Cargo produces from the library:

| `crate-type` | File | Contents | Used by |
| --- | --- | --- | --- |
| `lib` (`rlib`) | `libNAME.rlib` | Rust code and metadata | other Rust crates |
| `staticlib` | `libNAME.a`, `NAME.lib` | Your code, the Rust standard library and all Rust dependencies, as one archive | a C linker, at build time |
| `cdylib` | `libNAME.so`, `.dylib`, `NAME.dll` | The same as a shared library; only the exported C symbols are visible | a C linker, and the loader at run time |

A package can ask for several at once: `crate-type = ["lib", "staticlib"]`
gives the Rust tests their `rlib` and the C program its archive. A static
library contains the Rust standard library, but not the system libraries that
the standard library calls. rustc prints the list:

```text
$ cargo rustc --lib --crate-type staticlib -- --print native-static-libs
note: link against the following native artifacts when linking against this static library. The order and any duplication can be significant on some platforms

note: native-static-libs: -lSystem -lc -lm
```

That is the output on macOS. On Linux and on Windows the list is longer and
different; always take it from rustc for the target that you build for.

The last part is the **header**: the C declarations of what you export. The
situation of lesson 03 is now reversed, and it is not better. The C compiler
believes the header and never sees your Rust code. If the header says `int`
where the function returns `u64`, the program compiles, links, and reads half
of the result.

### Worked example: a step counter

The Rust side. The example also calls its own functions, so that it runs as a
test:

```rust
use std::ffi::c_int;

/// A step counter. Opaque to C.
pub struct Meter {
    steps: u64,
    limit: u64,
}

/// Creates a meter that counts up to `limit`. Never returns NULL.
#[unsafe(no_mangle)]
pub extern "C" fn meter_new(limit: u64) -> *mut Meter {
    Box::into_raw(Box::new(Meter { steps: 0, limit }))
}

/// Adds one step. Returns 0, or 1 when the limit is reached or `meter` is NULL.
///
/// # Safety
/// `meter` must be NULL or a live pointer from `meter_new` that no other
/// call is using.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn meter_step(meter: *mut Meter) -> c_int {
    // SAFETY: the caller promises NULL or a live meter that nobody else
    // uses; `as_mut` turns NULL into `None`.
    let Some(meter) = (unsafe { meter.as_mut() }) else {
        return 1;
    };
    if meter.steps == meter.limit {
        return 1;
    }
    meter.steps += 1;
    0
}

/// The number of steps so far; 0 when `meter` is NULL.
///
/// # Safety
/// `meter` must be NULL or a live pointer from `meter_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn meter_steps(meter: *const Meter) -> u64 {
    // SAFETY: the caller promises NULL or a live meter.
    unsafe { meter.as_ref() }.map_or(0, |meter| meter.steps)
}

/// Destroys a meter. `meter` may be NULL.
///
/// # Safety
/// `meter` must be NULL or a live pointer from `meter_new`, not used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn meter_free(meter: *mut Meter) {
    if !meter.is_null() {
        // SAFETY: a non-null pointer came from Box::into_raw in meter_new,
        // and the caller promises that this is its only release.
        drop(unsafe { Box::from_raw(meter) });
    }
}

fn main() {
    let meter = meter_new(2);
    // SAFETY: `meter` is live until `meter_free`, and used only here.
    unsafe {
        assert_eq!(meter_step(meter), 0);
        assert_eq!(meter_step(meter), 0);
        assert_eq!(meter_step(meter), 1); // the limit
        assert_eq!(meter_steps(meter), 2);
        assert_eq!(meter_step(std::ptr::null_mut()), 1);
        meter_free(meter);
    }
}
```

Three details carry the weight.

- `Meter` has no `repr(C)`. C only holds a pointer to it and never looks
  inside, so its layout is not part of the interface. That is the point of an
  opaque handle: you can add a field tomorrow without breaking any C program.
- `Box::into_raw` turns "Rust owns this" into "the C caller owns this".
  `Box::from_raw` in `meter_free` turns it back, once. Between the two, Rust
  sees only raw pointers and creates references for the length of one call.
- Every function that trusts a pointer is an `unsafe extern "C" fn` with a
  `# Safety` section. From Rust, calling it needs `unsafe`. From C there is no
  such word, so the same text must be in the header as a comment.

The header, written by hand:

```c
#ifndef METER_H
#define METER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* A step counter. Create with meter_new, release with meter_free. */
typedef struct meter meter;

/* Creates a meter that counts up to `limit`. Never returns NULL. */
meter *meter_new(uint64_t limit);

/* Adds one step. Returns 0, or 1 when the limit is reached or `m` is NULL. */
int meter_step(meter *m);

/* The number of steps so far; 0 when `m` is NULL. */
uint64_t meter_steps(const meter *m);

/* Destroys a meter. `m` may be NULL. Do not use it afterwards. */
void meter_free(meter *m);

#ifdef __cplusplus
}
#endif

#endif
```

The include guard stops a double inclusion, and the `extern "C"` block lets a
C++ program include the same header.

The tool [`cbindgen`](https://github.com/mozilla/cbindgen) writes such a
header from the Rust source. For the crate above, `cbindgen --lang c` (version
0.29.4) printed:

```text
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * A step counter. Opaque to C.
 */
typedef struct Meter Meter;

/**
 * Creates a meter that counts up to `limit`. Never returns NULL.
 */
struct Meter *meter_new(uint64_t limit);

/**
 * Adds one step. Returns 0, or 1 when the limit is reached or `meter` is NULL.
 *
 * # Safety
 * `meter` must be NULL or a live pointer from `meter_new` that no other
 * call is using.
 */
int meter_step(struct Meter *meter);
...
```

It copies your doc comments, which is one more reason to write the `# Safety`
sections well. It has no include guard unless you configure one.

### Strings and other memory that crosses outward

When an exported function returns memory to C, lesson 06's three questions
come back with the roles exchanged. There are three sound designs for
returning a string:

- **Rust allocates, and Rust frees through an exported function.**
  `CString::into_raw` gives the pointer away; a function such as
  `mylib_string_free` takes it back with `CString::from_raw`. The header must
  say two things. C's `free` is forbidden for this pointer. And the caller
  must not change the length of the string, for example by writing a NUL
  into it: `from_raw` finds the size of the allocation by searching for the
  NUL, and releasing memory with a wrong size is undefined behavior.
- **The caller provides the buffer** and its size, and gets the needed size
  back (lesson 06's model). No ownership crosses the boundary at all.
- **The library keeps the string** and returns a borrowed pointer that is
  valid until a documented later event. The library must really keep it alive
  that long.

### What you may change later: ABI stability

Once C programs are compiled against your header, everything that the header
states is frozen: symbol names, the number, order and types of parameters,
return types, the values of constants, and the layout of every struct that C
can see into. A change to any of them breaks old programs **without any error
message**, because the old program is not recompiled, or is recompiled against
an old header. What stays free:

- the fields of opaque types, because C never sees them;
- new functions and new constants, because old programs do not mention them;
- the meaning of a function only as far as its documentation left it open.

This is why C interfaces prefer opaque handles to public structs, return
status codes as `int` instead of an enum type, and often export a function
that returns an interface version number.

### Mistakes people make

**Two exports with the same name.** Symbol names are one flat namespace.
Within one crate the compiler notices (this is drill 10); across libraries
only the linker does, or nobody:

```text
error: symbol `init` is already defined
  --> src/main.rs:22:5
   |
22 |     pub extern "C" fn init() -> i32 {
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

Give every exported name a prefix that belongs to your library.

**A Rust-only type in an exported signature.** The compiler warns:

```text
warning: `extern` fn uses type `str`, which is not FFI-safe
 --> src/main.rs:2:31
  |
2 | pub extern "C" fn greet(name: &str) -> usize {
  |                               ^^^^ not FFI-safe
  |
  = help: consider using `*const u8` and a length instead
  = note: string slices have no C equivalent
  = note: `#[warn(improper_ctypes_definitions)]` on by default
```

**A function that the header declares and the library does not export.** The
C compiler is content; the linker is not. On macOS it reports (other linkers
word it differently):

```text
Undefined symbols for architecture arm64:
  "_meter_reset", referenced from:
      _main in use-6e292f.o
ld: symbol(s) not found for architecture arm64
```

Look for a spelling difference or a missing `#[unsafe(no_mangle)]`. Without
that attribute the function exists in the library, but under a mangled Rust
name that no header can spell.

**A panic that reaches the boundary.** An exported function is an
`extern "C"` function, so lesson 05's rule applies: an escaping panic aborts
the caller's process. Write exported functions so that they cannot panic
(checked arithmetic, no indexing, no `unwrap`), or catch at the boundary
(lesson 08).

### When to use which

- **`staticlib`** when the C program should be one file with no run-time
  dependency on your library. **`cdylib`** when several programs share the
  library, when it must be replaceable without relinking, or when a plug-in
  system loads it at run time.
- **A hand-written header** for a small, stable interface where you want
  control over every comment. **cbindgen** when the interface is large or
  changes often; then generate the header in CI and fail the build when the
  committed file differs.

### Read more

- [The Rust Reference: linkage and crate types](https://doc.rust-lang.org/reference/linkage.html).
- [The Rust Reference: the `no_mangle` attribute](https://doc.rust-lang.org/reference/abi.html#the-no_mangle-attribute).
- [The Rustonomicon: calling Rust code from C](https://doc.rust-lang.org/nomicon/ffi.html#calling-rust-code-from-c).
- [The Cargo book: the `crate-type` field](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-crate-type-field).

## Lesson 08 · Capstone: a Rust library with a C interface, built on a C library

**Assumed:** every lesson of this module.

The capstone has no new mechanism. It asks you to hold all of them at once: a
C program calls your Rust library, and your library calls a C library. This
section gives you two patterns that such a library needs and one picture to
keep in your head.

### A guard at every entry point

In lesson 07 you could write functions that cannot panic. A larger library
cannot promise that for all the code it calls. The standard answer is one
function that every exported function passes its body to:

```rust
use std::ffi::c_int;
use std::panic::catch_unwind;

const OK: c_int = 0;
const ERR_PANIC: c_int = -1;

/// `int percent(int part, int whole, int *out);`
///
/// # Safety
/// `out` must be NULL or valid for a write of one int.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn percent(part: c_int, whole: c_int, out: *mut c_int) -> c_int {
    // Division by zero panics. The panic stops here, inside Rust.
    match catch_unwind(|| part * 100 / whole) {
        Ok(value) => {
            if !out.is_null() {
                // SAFETY: a non-null `out` is valid for one write.
                unsafe { out.write(value) };
            }
            OK
        }
        Err(_payload) => ERR_PANIC,
    }
}

fn main() {
    let mut out = 0;
    // SAFETY: `&mut out` is valid for one write.
    assert_eq!(unsafe { percent(1, 4, &mut out) }, OK);
    assert_eq!(out, 25);
    // SAFETY: as above.
    assert_eq!(unsafe { percent(1, 0, &mut out) }, ERR_PANIC);
    assert_eq!(out, 25, "a failed call leaves the output alone");
}
```

Three questions decide whether such a guard is correct.

- *What state does a caught panic leave behind?* If the body was half way
  through changing an object, the object may be inconsistent. Either make each
  change a single step that cannot fail in the middle, or mark the object as
  unusable after a panic.
- *Can the guard itself panic?* The code in the `Err` arm runs outside
  `catch_unwind`. Keep it trivial. One thing in that arm is easy to miss:
  the payload. It is a value of any type, and dropping it runs that type's
  destructor. A careful guard drops the payload inside a second
  `catch_unwind`. `mem::forget(payload)` also keeps the destructor from
  running, but it leaks the payload, and Miri reports the leak.
- *Does the caller learn what happened?* A status code alone is little help.
  Hence the second pattern.

### An error message that C can read

A C function returns one `int`. To say more, C libraries keep the text of the
last error and export a function that returns it. The text must be stored
somewhere that lives after the failing call has returned, and two threads must
not overwrite each other's text. A `thread_local!` holding a `CString` does
both: the exported function returns `as_ptr()` of the stored string, and the
header says "valid until the next failed call on this thread". That sentence
is a borrow rule that no compiler checks, so it must be easy to follow, and
the C caller copies the text if it needs it longer.

### The picture

```text
one call of m6_stock_for_each, at the moment the visitor runs:

  main                        C     the test program
  m6_stock_for_each           Rust  your exported function, inside its guard
  Table::for_each             Rust  lesson 05
  kvtab_for_each              C     the vendored library
  the trampoline              Rust  lesson 05; catches panics
  your closure                Rust  decodes 8 bytes, then calls...
  visit                       C     the test program's callback
```

A panic may travel only through neighbouring Rust frames. It must stop at the
trampoline (below a C frame) and at the guard (below `main`). The C visitor at
the bottom cannot panic at all, and you must not let anything unwind into it
or out of it.

Before you call the capstone finished, go through this list for each exported
function:

1. Every pointer parameter: is NULL handled? What must the caller guarantee
   beyond that, and does the header say so?
2. Every `unsafe` block: does the SAFETY comment name the promise it relies
   on, and who made that promise (the header, the invariant of a type, or a
   check a few lines above)?
3. Every output: on an error path, is it left unwritten, as the contract says?
4. Every allocation that crosses the boundary: who frees it, with what?
5. Every path: can a panic start here, and where does it stop?

### Read more

- [`std::thread_local!`](https://doc.rust-lang.org/std/macro.thread_local.html) and [`LocalKey::try_with`](https://doc.rust-lang.org/std/thread/struct.LocalKey.html#method.try_with).
- [`std::panic::catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html): what it does and does not guarantee.
- [The Rustonomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html), read again from the start as a checklist.
