# Chapter 24 concepts — Calling C

[Chapter guide](README.md) · [Hints](HINTS.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. Every code block in this file was compiled and
run with the course toolchain (Rust 1.99.0, edition 2024), and every compiler
message was copied from that compiler. Long messages are shortened: a line with
`...` marks a cut, and the end of a message is often left out. File names in
messages are written as `src/main.rs`.

## What you will be able to do

After this chapter you can:

- describe a C struct, a C string and a C function to Rust so that both sides
  agree on every byte;
- call a function from a C library and say exactly which promises you made by
  writing `unsafe`;
- wrap a C-style interface (status codes, output pointers, callbacks, handles)
  in a safe Rust interface that a caller cannot misuse;
- give a borrowed foreign pointer a Rust lifetime, so that the compiler rejects
  a use after free;
- read the lints and errors that the compiler has for foreign code, and know
  which mistakes it cannot see at all.

## Before you start

Chapter 23 is required: raw pointers, `unsafe`
blocks, `// SAFETY:` comments, `MaybeUninit`, `size_of` and `align_of`. You
also need ownership and `Drop` (Chapters 4 and 13), references and lifetimes
(Chapter 5), structs and enums (Chapters 9 and 10), error handling with
`Result` (Chapter 7) and `String` against `&str` (Chapter 17). You do not need
to know C. Each C declaration in this chapter is explained where it appears.

**Foreign function interface** (FFI) is the name for the rules that let code
in one language call code in another. In this chapter the other language is
C, because nearly every language can speak C's calling rules.

In lessons 01 to 07 the "foreign" functions are written in Rust with C's
calling rules, so that every test runs without a C compiler. In lesson 08 you
call functions that really are compiled C code.

## Time

These are estimates for a learner who has finished Chapter 23. They
include reading the section, the predictions, the code, and your notes.

| Lesson | Estimate |
| --- | --- |
| 01 · C representations | 35 minutes |
| 02 · C strings and ownership | 45 minutes |
| 03 · ABI calls and out-parameters | 45 minutes |
| 04 · Callbacks and userdata | 50 minutes |
| 05 · Opaque handle ownership | 50 minutes |
| 06 · Owner-bound foreign borrows | 50 minutes |
| 08 · Calling the real C library | 70 minutes |
| 07 · Capstone | 60 minutes |
| R01 review, C01 code review, drills | 120 minutes |

## Words used in this chapter

- **ABI** (application binary interface): the machine-level rules for a call.
  They say where arguments and results are placed, and how each type is laid
  out in memory. `extern "C"` selects the C ABI of the target platform.
- **Layout**: the size of a type in bytes, its alignment, and the position of
  each field.
- **Alignment**: a number such as 1, 2, 4 or 8. A value of the type must start
  at an address that is a multiple of it.
- **Padding**: unused bytes that the compiler puts between fields, or at the
  end of a struct, so that every field is aligned.
- **NUL**: the byte with value 0. C marks the end of a string with it.
- **Out-parameter**: a pointer argument through which a function gives a
  result back to its caller.
- **Userdata** (also "context"): a pointer that a C library does not look at.
  It only passes the pointer back to your callback.
- **Handle**: a pointer to an object that only the library can look inside.
- **Undefined behavior** (UB): the program broke a rule that the compiler
  relies on. Anything may happen, including a result that looks correct.

## Lesson 01 · C representations

This section assumes the integer types of Chapter 3, structs from Chapter 9,
and `size_of` and `align_of` from Chapter 23.

### The idea

A C library and your Rust program are compiled by different compilers.
Neither compiler sees the other's source code. At the boundary there are only
bytes, so both sides must agree on three things: how many bytes each value
has, how it is aligned, and in which order the fields of a struct lie.

C's integer types have no fixed size. The C standard gives only minimum
sizes, and each platform chooses. So `std::ffi` has one alias for each C
type. An alias is whatever the C compiler of your target uses.

| C type | Rust | On today's desktop, server and phone targets |
| --- | --- | --- |
| `char` | `c_char` | 1 byte; signed on some targets, unsigned on others |
| `int`, `unsigned int` | `c_int`, `c_uint` | 4 bytes |
| `long` | `c_long` | 8 bytes on 64-bit Linux and macOS, 4 bytes on Windows |
| `long long` | `c_longlong` | 8 bytes |
| `double` | `c_double` | 8 bytes |
| `size_t` | `usize` | the size of a pointer |
| `void *` | `*mut c_void` | a pointer to "something" |

Types with a fixed width, such as C's `uint8_t` and `int32_t`, map to `u8` and
`i32` directly.

### Worked example: a struct that C can read

Rust is free to reorder the fields of an ordinary struct. `#[repr(C)]` turns
that freedom off: the fields are laid out in the order you wrote them, with
the same rules as C.

```rust
use std::mem::{align_of, offset_of, size_of};

#[repr(C)]
struct Packet {
    kind: u8,
    length: u32,
    port: u16,
}

assert_eq!(offset_of!(Packet, kind), 0);
assert_eq!(offset_of!(Packet, length), 4);
assert_eq!(offset_of!(Packet, port), 8);
assert_eq!(size_of::<Packet>(), 12);
assert_eq!(align_of::<Packet>(), 4);
```

```text
byte    0      1   2   3     4   5   6   7     8   9     10  11
      [ kind | padding    | length          | port    | padding ]
```

Two rules give this picture. First, each field starts at the next address
that is a multiple of its own alignment: `length` has alignment 4, so three
padding bytes follow `kind`. Second, the alignment of the struct is the
largest alignment among its fields, and the size is rounded up to a multiple
of it: 10 bytes become 12. Without `#[repr(C)]`, the course compiler orders
the same three fields so that they take 8 bytes. That is allowed, and it is
exactly why a struct without `#[repr(C)]` must not cross the boundary.

`#[repr(C)]` describes the layout of the struct. It says nothing about the
types of the fields. A `String` field inside a `#[repr(C)]` struct is still a
Rust `String`, and C cannot use it.

### Enums cross the boundary as integers

`#[repr(u8)]` on an enum without fields fixes its size to one byte, and
`#[repr(C)]` gives it the size that a C `enum` has. In both cases a Rust enum
may only ever hold one of its declared values. C can send any integer. So
receive the integer, and convert it yourself:

```rust
#[repr(u8)]
#[derive(Debug, PartialEq)]
enum Mode {
    Idle = 0,
    Busy = 1,
}

fn mode_from_c(raw: u8) -> Option<Mode> {
    match raw {
        0 => Some(Mode::Idle),
        1 => Some(Mode::Busy),
        _ => None,
    }
}

assert_eq!(mode_from_c(1), Some(Mode::Busy));
assert_eq!(mode_from_c(9), None);
assert_eq!(Mode::Busy as u8, 1); // from enum to integer, `as` is fine
```

### Mistakes people make

**Putting a Rust-only type in a foreign signature.** The code compiles, with
a warning. (An `unsafe extern` block declares functions that live in another
library. Lesson 08 explains it.)

```rust
use std::ffi::c_int;

unsafe extern "C" {
    fn submit_label(label: String, code: c_int) -> c_int;
}

fn main() {}
```

```text
warning: `extern` block uses type `String`, which is not FFI-safe
 --> src/main.rs:4:28
  |
4 |     fn submit_label(label: String, code: c_int) -> c_int;
  |                            ^^^^^^ not FFI-safe
  |
  = help: consider adding a `#[repr(C)]` or `#[repr(transparent)]` attribute to this struct
  = note: this struct has unspecified layout
  = note: `#[warn(improper_ctypes)]` on by default
```

Fix: pass what C understands. For text, that is a pointer to a C string
(Lesson 02). Do not follow the `help` here: you cannot add an attribute to
`String`.

**Casting an integer to an enum.**

```rust,compile_fail,E0605
#[repr(u8)]
#[derive(Debug)]
enum Mode {
    Idle = 0,
    Busy = 1,
}

fn main() {
    let from_c: u8 = 1;
    let mode = from_c as Mode;
    println!("{mode:?}");
}
```

```text
error[E0605]: non-primitive cast: `u8` as `Mode`
  --> src/main.rs:10:16
   |
10 |     let mode = from_c as Mode;
   |                ^^^^^^^^^^^^^^ an `as` expression can only be used to convert between primitive types or to coerce to a specific trait object
```

Fix: a `match`, as in `mode_from_c` above. Do not reach for `transmute`: a
`Mode` that holds the value 9 is undefined behavior.

**Using a struct's memory as a file format.** The bytes of `Packet` are not
the same on every machine: sizes and byte order differ between targets, and
padding bytes have no defined value at all. To send a record over a network
or to write it to a file, write each field yourself in a byte order that you
choose. The review lesson of this chapter has you do that.

**Assuming the size of a C type.** Code that stores a C `long` in an `i64`
works on Linux and breaks on Windows. Write `c_long` and let the alias be
right on every target.

### Read more

- [The `C` representation, in the Rust Reference](https://doc.rust-lang.org/reference/type-layout.html#the-c-representation)
- [The `std::ffi` module](https://doc.rust-lang.org/std/ffi/index.html)
- [`std::mem::offset_of!`](https://doc.rust-lang.org/std/mem/macro.offset_of.html)

## Lesson 02 · C strings and ownership

This section assumes `String` and `&str` from Chapter 17 and the layout ideas
of Lesson 01.

### The idea

A Rust `&str` is two things: a pointer and a length. Its bytes are valid
UTF-8. Because the length is stored, the byte 0 has no special meaning: it
may appear anywhere in the text.

A C string is one thing: a pointer to the first byte. There is no length. The
string ends at the first NUL byte, so a C string can never contain a NUL in
the middle. C does not promise any text encoding.

```text
Rust &str "ok"           pointer ──► [ o | k ]
                         length 2

C string "ok"            pointer ──► [ o | k | 0 ]
                                               └ the NUL marks the end
```

The standard library has one pair of types for this, in the same relation as
`String` and `&str`:

- `CString` **owns** a heap buffer that ends with a NUL. You build one when
  Rust text must go to C.
- `&CStr` **borrows** such a buffer. You get one when you look at a string
  that C (or a `CString`) owns.

### Worked example

```rust
use std::ffi::{CStr, CString};

// Rust text going to C: build an owner. This allocates and adds the NUL.
let owned: CString = CString::new(format!("sensor-{}", 7)).unwrap();
assert_eq!(owned.as_bytes(), b"sensor-7");
assert_eq!(owned.as_bytes_with_nul(), b"sensor-7\0");

// A CString can be borrowed as a &CStr, like a String as a &str.
let view: &CStr = &owned;
assert_eq!(view.to_str(), Ok("sensor-7"));

// Text coming from C is only bytes. Here the byte 0xE9 is not valid UTF-8.
let from_c: &CStr = c"caf\xe9";
assert!(from_c.to_str().is_err());
assert_eq!(from_c.to_string_lossy(), "caf\u{fffd}");
assert_eq!(from_c.to_bytes(), b"caf\xe9");
```

`c"..."` is a C string literal. Its type is `&'static CStr`: the compiler
adds the NUL and stores the bytes in the program, so nothing is allocated.

To hand a string to C you call `as_ptr()`, which gives a `*const c_char`. A
raw pointer has no lifetime (Chapter 23). From that moment, keeping the
buffer alive is your job, not the compiler's.

### Mistakes people make

**Taking a pointer from a temporary `CString`.**

```rust
use std::ffi::{CString, c_char};

fn main() {
    let p: *const c_char = CString::new("temp").unwrap().as_ptr();
    println!("{p:?}");
}
```

```text
warning: this creates a dangling pointer because temporary `CString` is dropped at end of statement
 --> src/main.rs:4:58
  |
4 |     let p: *const c_char = CString::new("temp").unwrap().as_ptr();
  |                            ----------------------------- ^^^^^^ pointer created here
  |                            |
  |                            this `CString` is dropped at end of statement
  |
  = help: bind the `CString` to a variable such that it outlives the pointer returned by `as_ptr`
```

This is only a warning, and the program runs. The buffer is freed at the
semicolon, so `p` points at freed memory. Fix: give the owner a name, and
keep that variable alive until C has finished with the pointer.

```rust
use std::ffi::CString;

let owner = CString::new("temp").unwrap();
let p = owner.as_ptr(); // valid for as long as `owner` lives and is not changed
assert!(!p.is_null());
```

**Calling `unwrap` on `CString::new` with text that you did not write.**

```rust,should_panic
use std::ffi::CString;

fn main() {
    let user_input = "report\0.txt";
    let name = CString::new(user_input).unwrap();
    println!("{name:?}");
}
```

```text
thread 'main' (...) panicked at src/main.rs:5:41:
called `Result::unwrap()` on an `Err` value: NulError(6, [114, 101, 112, 111, 114, 116, 0, 46, 116, 120, 116])
```

`CString::new` checks for a NUL in the middle, because C would silently stop
reading there. The check can fail, so the result is a `Result`. Fix: handle
the error, or return it to your caller, as you learned in Chapter 7. The
error type is `NulError`, and it knows where the NUL was found.

```rust
use std::ffi::CString;

fn describe(input: &str) -> String {
    match CString::new(input) {
        Ok(name) => format!("{} bytes for C", name.as_bytes_with_nul().len()),
        Err(error) => format!("NUL byte at index {}", error.nul_position()),
    }
}

assert_eq!(describe("report.txt"), "11 bytes for C");
assert_eq!(describe("report\0.txt"), "NUL byte at index 6");
```

**Treating C text as UTF-8 without a check.** `to_str` checks and can fail.
`to_string_lossy` never fails, and replaces each bad sequence with U+FFFD.
Both only look at the bytes. To keep the text after C frees or reuses its
buffer, you must copy it into a `String`.

### When to use which

| You have | You need | Use |
| --- | --- | --- |
| Rust text | an argument for C | `CString::new(text)?`, then `.as_ptr()` |
| a fixed text in your source | an argument for C | a literal, `c"text"` |
| a pointer from C that is valid now | to read it | `unsafe { CStr::from_ptr(p) }` |
| a byte slice that should end with one NUL | a checked `&CStr` | `CStr::from_bytes_with_nul(bytes)` |
| a byte buffer with a NUL somewhere in it | the part before that NUL | `CStr::from_bytes_until_nul(bytes)` |
| a `&CStr` | Rust text that outlives the C buffer | `.to_str()?.to_owned()`, or `.to_string_lossy().into_owned()` |

The two `from_bytes` constructors never read outside the slice. `from_ptr`
reads until it finds a NUL, wherever that is, and for that reason it is
`unsafe`.

### Read more

- [`CString`](https://doc.rust-lang.org/std/ffi/struct.CString.html) and
  [`CStr`](https://doc.rust-lang.org/std/ffi/struct.CStr.html)
- [C string literals, in the Rust Reference](https://doc.rust-lang.org/reference/tokens.html#c-string-and-raw-c-string-literals)
- [`CString::as_ptr` and the dangling-pointer warning](https://doc.rust-lang.org/std/ffi/struct.CString.html#method.as_ptr)

## Lesson 03 · ABI calls and out-parameters

This section assumes `MaybeUninit` and `unsafe fn` from Chapter 23, and
`Result` from Chapter 7.

### The idea

Writing `extern "C"` in front of `fn` changes one thing: the function uses
the C calling rules of the platform instead of Rust's own. A function that is
defined this way in Rust can be called from C, and it has the same shape as a
function that C defines. `extern "C"` does not make a function safe or
unsafe. That is a separate word.

C has no `Result` and no tuples. A C function that can fail usually returns
an integer **status**, and gives its real result back through a pointer that
the caller supplies: an **out-parameter**. The usual contract is:

- status 0 means success, and the function has written the result;
- any other status means failure, and the result slot was **not** written.

So after a failure the slot may still be uninitialized memory. Reading it is
undefined behavior. `MaybeUninit<T>` is the type for "room for a `T` that may
not hold a `T` yet".

### Worked example

Here is a foreign-style function that turns one hexadecimal digit into its
value, followed by a safe wrapper.

```rust
use std::ffi::c_int;
use std::mem::MaybeUninit;

/// Writes the value of a hexadecimal digit to `out` and returns 0.
/// Returns 1 and leaves `out` untouched if `digit` is not a digit.
///
/// # Safety
/// `out` must be valid for writing one `u8`.
unsafe extern "C" fn hex_value(digit: u8, out: *mut u8) -> c_int {
    let value = match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        _ => return 1,
    };
    // SAFETY: the caller passes a writable slot.
    unsafe { out.write(value) };
    0
}

fn hex(digit: u8) -> Option<u8> {
    let mut slot = MaybeUninit::<u8>::uninit();
    // SAFETY: `slot` is room for one u8, and it is writable.
    let status = unsafe { hex_value(digit, slot.as_mut_ptr()) };
    if status == 0 {
        // SAFETY: status 0 means that hex_value wrote the slot.
        Some(unsafe { slot.assume_init() })
    } else {
        None // the slot was never written, and it is never read
    }
}

assert_eq!(hex(b'c'), Some(12));
assert_eq!(hex(b'0'), Some(0)); // 0 is a result, not a failure
assert_eq!(hex(b'z'), None);
```

```text
      hex                          hex_value
  ┌──────────────┐   pointer    ┌─────────────────────────────┐
  │ slot: ? ? ?  │ ◄─────────── │ success: writes the slot     │
  │ status       │ ◄─ returned ─│ failure: does not touch it   │
  └──────────────┘              └─────────────────────────────┘
  read the slot only if status == 0
```

The wrapper does three jobs that every safe wrapper does. It makes the
storage. It keeps the unsafe call inside, with a reason. And it turns C's
convention (a status and a slot) into Rust's (`Option` or `Result`), so that
the caller cannot read the wrong one.

### Function pointers carry their ABI

The type of a pointer to `hex_value` is
`unsafe extern "C" fn(u8, *mut u8) -> c_int`. The words `extern "C"` are part
of the type. A pointer to an ordinary Rust function has a different type,
even when the parameters are the same, because the two kinds of function
expect their arguments in different places. The compiler refuses to mix them
with error `E0308`. The compiler lab of this lesson asks you to produce that
message yourself.

### Mistakes people make

**Passing a variable without a value as the slot.**

```rust,compile_fail,E0381
use std::ffi::c_int;

unsafe extern "C" fn hex_value(digit: u8, out: *mut u8) -> c_int {
    let value = match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        _ => return 1,
    };
    // SAFETY: the caller passes a writable slot.
    unsafe { out.write(value) };
    0
}

fn main() {
    let out: u8;
    let status = unsafe { hex_value(b'c', &mut out) };
    println!("{status} {out}");
}
```

```text
error[E0381]: used binding `out` isn't initialized
  --> src/main.rs:16:43
   |
15 |     let out: u8;
   |         --- binding declared here but left uninitialized
16 |     let status = unsafe { hex_value(b'c', &mut out) };
   |                                           ^^^^^^^^ `out` used here but it isn't initialized
```

The compiler cannot know that the function will write through the pointer.
There are two fixes. `MaybeUninit`, as above, says exactly what is true. Or
give the variable a starting value such as 0. The second fix has a cost: if
you forget to check the status, your program continues with the 0 and
nothing tells you.

**Calling the function without `unsafe`.**

```rust,compile_fail,E0133
use std::ffi::c_int;

unsafe extern "C" fn hex_value(digit: u8, out: *mut u8) -> c_int {
    let value = match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        _ => return 1,
    };
    // SAFETY: the caller passes a writable slot.
    unsafe { out.write(value) };
    0
}

fn main() {
    let mut out = 0_u8;
    let status = hex_value(b'c', &mut out);
    println!("{status} {out}");
}
```

```text
error[E0133]: call to unsafe function `hex_value` is unsafe and requires unsafe block
  --> src/main.rs:16:18
   |
16 |     let status = hex_value(b'c', &mut out);
   |                  ^^^^^^^^^^^^^^^^^^^^^^^^^ call to unsafe function
   |
   = note: consult the function's documentation for information on how to avoid undefined behavior
```

Fix: an `unsafe` block with a `// SAFETY:` comment that answers the
function's `# Safety` section point by point.

**Treating "not zero" and "negative" as the same thing.** Many C functions
use positive codes too. Read the contract, and compare with the one value
that means success.

### Read more

- [`MaybeUninit`, with a section on out-pointers](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html#out-pointers)
- [The `extern` function qualifier, in the Rust Reference](https://doc.rust-lang.org/reference/items/functions.html#extern-function-qualifier)
- [Function pointer types](https://doc.rust-lang.org/reference/types/function-pointer.html)

## Lesson 04 · Callbacks and userdata

This section assumes function pointers from Lesson 03, raw pointer casts from
Chapter 23, and closures from Chapter 14.

### The idea

Sometimes the C library calls **you**. You give it a function pointer, and it
calls that function once for each item, event or line. Such a function is a
**callback**.

A C function pointer is only an address. It cannot carry data, as a Rust
closure can. So C libraries use a fixed pattern: next to the function pointer
you pass one more pointer, of type `void *`. The library does not look at it.
It passes it back to your callback on every call. This pointer is the
**userdata** (or "context"). In Rust its type is `*mut c_void`, and your
callback casts it back to the real type.

The important question is: **how long does the library keep the pointer?**

- A **synchronous** library uses the callback only during the call that
  received it. A local variable is then a fine userdata: it outlives the call.
- A library that **stores** the callback for later needs userdata that lives
  until you unregister it. That usually means a `Box` that you turn into a raw
  pointer with `Box::into_raw` and take back later with `Box::from_raw`.

### Worked example

```rust
use std::ffi::{c_int, c_void};

type Visitor = unsafe extern "C" fn(c_int, *mut c_void);

/// Calls `visit(n, ctx)` for every n in `from..to`, in order.
///
/// # Safety
/// `visit` must accept `ctx`. Neither is kept after this function returns.
unsafe extern "C" fn visit_range(from: c_int, to: c_int, visit: Visitor, ctx: *mut c_void) {
    for n in from..to {
        // SAFETY: the caller promises that visit accepts ctx.
        unsafe { visit(n, ctx) };
    }
}

struct Stats {
    count: usize,
    largest: c_int,
}

/// # Safety
/// `ctx` must point to a live `Stats` that nothing else uses during the call.
unsafe extern "C" fn record(n: c_int, ctx: *mut c_void) {
    // SAFETY: promised by the caller.
    let stats = unsafe { &mut *ctx.cast::<Stats>() };
    stats.count += 1;
    stats.largest = stats.largest.max(n);
}

fn range_stats(from: c_int, to: c_int) -> Stats {
    let mut stats = Stats { count: 0, largest: c_int::MIN };
    // SAFETY: `record` reads its context as a Stats, and that is what we
    // pass. `stats` lives until the end of this function, and visit_range
    // keeps no pointer after it returns.
    unsafe { visit_range(from, to, record, (&raw mut stats).cast()) };
    stats
}

let stats = range_stats(3, 8);
assert_eq!((stats.count, stats.largest), (5, 7));
```

```text
range_stats                     visit_range                 record
┌───────────────────┐  ctx     ┌──────────────┐  ctx     ┌──────────────────┐
│ stats ◄───────────┼──────────┤ passes it on ├──────────┤ casts it back to │
│  count, largest   │          │ unchanged    │          │ *mut Stats       │
└───────────────────┘          └──────────────┘          └──────────────────┘
`stats` must stay alive for as long as the library may call `record`.
```

`&raw mut stats` makes a raw pointer to `stats`, and `.cast()` changes the
type that it points to. Nothing checks the cast back inside `record`. If you
pass a pointer to something else, the program has undefined behavior, and the
compiler says nothing. That is why both functions have a `# Safety` section.

### A callback that may be absent

In C, "no callback" is a null function pointer. A Rust function pointer is
never null. The Rust type for "a function or nothing" is
`Option<extern "C" fn(...)>`. It has the same size as the bare pointer,
`None` is C's null, and the compiler makes you handle the `None` case.

### Mistakes people make

**Passing a closure that uses a local variable.**

```rust,compile_fail,E0308
use std::ffi::{c_int, c_void};

type Visitor = unsafe extern "C" fn(c_int, *mut c_void);

unsafe extern "C" fn visit_range(from: c_int, to: c_int, visit: Visitor, ctx: *mut c_void) {
    for n in from..to {
        // SAFETY: the caller promises that visit accepts ctx.
        unsafe { visit(n, ctx) };
    }
}

fn main() {
    let mut total = 0;
    let add = |n: c_int, _ctx: *mut c_void| total += n;
    // SAFETY: (this call does not compile)
    unsafe { visit_range(1, 4, add, std::ptr::null_mut()) };
    println!("{total}");
}
```

```text
error[E0308]: mismatched types
  --> src/main.rs:16:32
   |
14 |     let add = |n: c_int, _ctx: *mut c_void| total += n;
   |               ----------------------------- the found closure
15 |     // SAFETY: (this call does not compile)
16 |     unsafe { visit_range(1, 4, add, std::ptr::null_mut()) };
   |              -----------       ^^^ expected fn pointer, found closure
   ...
note: closures can only be coerced to `fn` types if they do not capture any variables
```

A closure that captures `total` is a function **plus data**, and a C function
pointer has no room for the data. Fix: put the data behind the userdata
pointer, as `record` does with `Stats`.

**Letting a panic leave the callback.** A panic unwinds the stack, and C code
has no idea what unwinding is. So Rust stops the process when a panic reaches
the edge of an `extern "C"` function.

```rust,no_run
use std::ffi::c_int;

extern "C" fn on_value(n: c_int) {
    let divisor = n - 3;
    println!("{}", 12 / divisor);
}

fn main() {
    for n in 1..=3 {
        on_value(n);
    }
    println!("never printed");
}
```

```text
-6
-12

thread 'main' (...) panicked at src/main.rs:5:20:
attempt to divide by zero
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'main' (...) panicked at .../library/core/src/panicking.rs:225:5:
panic in a function that cannot unwind
...
thread caused non-unwinding panic. aborting.
```

The process is killed. The code after the call never runs, the destructors
of the callers never run, and in a test run every other test dies too. Fix: write callbacks that cannot panic (`checked_div`,
`saturating_mul`, `get` instead of indexing). When a callback must run code
that you do not control, wrap that code in
[`std::panic::catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)
and report the failure through the userdata.

**Userdata that dies too early.** If the library stores the pointer and your
function returns, a pointer to a local variable is left dangling. Nothing
warns you. Before you write the call, find the sentence in the library's
documentation that says how long the pointer is kept.

### Read more

- [Callbacks from C code to Rust functions, in the Rustonomicon](https://doc.rust-lang.org/nomicon/ffi.html#callbacks-from-c-code-to-rust-functions)
- [FFI and unwinding, in the Rustonomicon](https://doc.rust-lang.org/nomicon/ffi.html#ffi-and-unwinding)
- [The nullable pointer optimization](https://doc.rust-lang.org/nomicon/ffi.html#the-nullable-pointer-optimization)

## Lesson 05 · Opaque handle ownership

This section assumes `Drop` from Chapter 13, `Box::into_raw` and
`Box::from_raw` from Chapter 23, and C strings from Lesson 02.

### The idea

Many C libraries hide their data. You call a "create" function and get a
pointer back. You cannot look at what it points to. You can only pass the
pointer to other functions of the same library. In the end you must pass it
to the matching "destroy" function, exactly once. Such a pointer is an
**opaque handle**.

C checks none of this. You can forget to destroy a handle (a leak), destroy
it twice (a double free), or use it after destroying it. In Rust you can make
all three mistakes impossible for the users of your wrapper, with tools that
you already know:

- a struct that owns the handle, with a **private** field;
- `Drop`, which calls the destroy function;
- **no** `Clone` and no `Copy`, so that there is exactly one owner.

### Worked example

The "library" below keeps a counter. Imagine that the three `counter_`
functions are written in C and that you cannot see inside `Counter`.

```rust
use std::ptr::NonNull;

// ---- the library ----
struct Counter {
    value: u32,
}
fn counter_new() -> *mut Counter {
    Box::into_raw(Box::new(Counter { value: 0 }))
}
/// # Safety
/// `c` is a live handle from `counter_new`.
unsafe extern "C" fn counter_add(c: *mut Counter, n: u32) -> u32 {
    // SAFETY: promised by the caller.
    let counter = unsafe { &mut *c };
    counter.value = counter.value.saturating_add(n);
    counter.value
}
/// # Safety
/// `c` is a live handle from `counter_new`. It must not be used afterwards.
unsafe extern "C" fn counter_free(c: *mut Counter) {
    // SAFETY: the handle came from Box::into_raw in counter_new.
    unsafe { drop(Box::from_raw(c)) };
}

// ---- the safe wrapper ----
struct Tally {
    raw: NonNull<Counter>,
}

impl Tally {
    fn new() -> Option<Tally> {
        // A real create function can fail and return null. NonNull::new
        // turns null into None, so a Tally never holds a null handle.
        NonNull::new(counter_new()).map(|raw| Tally { raw })
    }
    fn add(&mut self, n: u32) -> u32 {
        // SAFETY: `raw` is live from `new` until `drop`, and `&mut self`
        // shows that nobody else is using the handle now.
        unsafe { counter_add(self.raw.as_ptr(), n) }
    }
}

impl Drop for Tally {
    fn drop(&mut self) {
        // SAFETY: this is the only owner, and drop runs once.
        unsafe { counter_free(self.raw.as_ptr()) };
    }
}

let mut tally = Tally::new().expect("the mock never fails");
tally.add(2);
assert_eq!(tally.add(3), 5);
// `tally` goes out of scope here, and counter_free runs exactly once
```

```text
 Tally (Rust)                         memory that the library owns
┌──────────────────┐
│ raw: NonNull ────┼────────────────► Counter { value: 5 }
└──────────────────┘
  one owner, no Clone                 created by counter_new
  Drop calls counter_free             freed only by counter_free
```

`NonNull<T>` is a raw pointer that is never null. It records a fact in the
type, and it lets `Option<Tally>` have the same size as `Tally`.

### Every allocation goes back to where it came from

Memory must be freed by the same allocator that made it. The mock uses `Box`
on both sides, so the pair matches. A real C library allocates with its own
allocator, and it gives you its own free function for that reason. Passing a
C pointer to `Box::from_raw`, or a Rust pointer to C's `free`, is undefined
behavior, even if it seems to work on your machine.

### Mistakes people make

**Using a handle after it was closed.** In C this compiles. With an owning
wrapper, the rule from Chapter 4 catches it.

```rust,compile_fail,E0382
struct Tally {
    raw: *mut u32,
}

impl Tally {
    fn new() -> Self {
        Tally { raw: Box::into_raw(Box::new(0)) }
    }
    fn get(&self) -> u32 {
        // SAFETY: raw is live until drop.
        unsafe { *self.raw }
    }
}

impl Drop for Tally {
    fn drop(&mut self) {
        // SAFETY: raw came from Box::into_raw and is freed only here.
        unsafe { drop(Box::from_raw(self.raw)) };
    }
}

fn main() {
    let tally = Tally::new();
    drop(tally);
    println!("{}", tally.get());
}
```

```text
error[E0382]: borrow of moved value: `tally`
  --> src/main.rs:25:20
   |
23 |     let tally = Tally::new();
   |         ----- move occurs because `tally` has type `Tally`, which does not implement the `Copy` trait
24 |     drop(tally);
   |          ----- value moved here
25 |     println!("{}", tally.get());
   |                    ^^^^^ value borrowed here after move
```

This error is the wrapper doing its job.

**Adding `#[derive(Clone)]` to make such an error go away.** The compiler
often suggests `Clone` in the rest of that message. For a handle it is wrong.
The derive copies the pointer, not the object. Now two owners exist, both run
`Drop`, and the handle is freed twice. No compiler message warns you. If you
need two owners, share one wrapper with `Rc` or `Arc` (Chapter 4), or use the
library's own function for taking another reference, if it has one.

**Calling the destructor by hand.**

```rust,compile_fail,E0040
struct Tally {
    raw: *mut u32,
}

impl Drop for Tally {
    fn drop(&mut self) {
        // SAFETY: raw came from Box::into_raw and is freed only here.
        unsafe { drop(Box::from_raw(self.raw)) };
    }
}

fn main() {
    let tally = Tally { raw: Box::into_raw(Box::new(0)) };
    tally.drop();
}
```

```text
error[E0040]: explicit use of destructor method
  --> src/main.rs:14:11
   |
14 |     tally.drop();
   |           ^^^^ explicit destructor calls not allowed
   |
help: consider using `drop` function
```

If you could call `Drop::drop` yourself, it would run again at the end of the
scope. `drop(tally)` moves the value, so its destructor runs once.

**Doing work that can fail after the handle exists.** If a function creates
a handle and then returns early with `?`, a bare raw pointer leaks. Either
check everything that can fail first, or put the handle into its owning
wrapper at once, so that the early return drops it.

### Read more

- [Representing opaque structs, in the Rustonomicon](https://doc.rust-lang.org/nomicon/ffi.html#representing-opaque-structs)
- [`NonNull`](https://doc.rust-lang.org/std/ptr/struct.NonNull.html)
- [`Box::into_raw`](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.into_raw) and
  [`Box::from_raw`](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.from_raw)

## Lesson 06 · Owner-bound foreign borrows

This section assumes lifetimes from Chapter 5 and the owning wrapper of
Lesson 05.

### The idea

C libraries often give you a pointer **into** an object that you already
hold: the name of a file handle, the text of an error, one row of a result.
The documentation then says something like "valid until the parent object is
closed". You do not own the thing behind this pointer. You must not free it,
and you must stop using it when its parent goes away.

That sentence describes a borrow. Rust can check borrows, but only if the
type has a lifetime, and a raw pointer has none. `PhantomData` adds one.
`PhantomData<&'a Parent>` is a field that takes no space. It tells the
compiler: "treat this struct as if it held a `&'a Parent`".

### Worked example

```rust
use std::ffi::c_int;
use std::marker::PhantomData;

struct Catalog {
    prices: Vec<c_int>,
}

/// A view of one price. The pointer goes into the catalog's buffer.
struct PriceRef<'c> {
    raw: *const c_int,
    owner: PhantomData<&'c Catalog>,
}

impl Catalog {
    fn first(&self) -> Option<PriceRef<'_>> {
        // The lifetime of `&self` flows into the result: '_ is that lifetime.
        self.prices.first().map(|price| PriceRef {
            raw: std::ptr::from_ref(price),
            owner: PhantomData,
        })
    }
}

impl PriceRef<'_> {
    fn get(&self) -> c_int {
        // SAFETY: `raw` came from a live element of the catalog. The catalog
        // stays borrowed for as long as this PriceRef exists, so nobody can
        // drop it or change its buffer in the meantime.
        unsafe { *self.raw }
    }
}

let catalog = Catalog { prices: vec![250, 990] };
let price = catalog.first().expect("the catalog is not empty");
assert_eq!(price.get(), 250);
assert_eq!(size_of::<PriceRef<'_>>(), size_of::<*const c_int>()); // no cost
```

```text
catalog ──owns──► [ 250 | 990 ]        (heap buffer)
                    ▲
price.raw ──────────┘                  a raw pointer: no lifetime of its own
price.owner: PhantomData<&'c Catalog>  the compiler sees a borrow of `catalog`
```

The borrow checker now treats `price` exactly like a `&Catalog`. While
`price` is in use, the catalog cannot be dropped, moved or changed. The
`unsafe` block in `get` relies on that, and its SAFETY comment says so.

### Borrow or copy?

A view is cheap, and it keeps its owner borrowed. A copy costs an allocation
or a few bytes, and it is free of the owner. For the price, `price.get()`
returns a copy of the number. For a C string, `.to_owned()` gives a copy
that can outlive the handle. Offer the view when callers usually look and
move on. Offer the copy when they keep the data.

### Mistakes people make

**Letting the view outlive its owner.** With the types above, this cannot
compile:

```rust,compile_fail,E0597
use std::ffi::c_int;
use std::marker::PhantomData;

struct Catalog {
    prices: Vec<c_int>,
}

struct PriceRef<'c> {
    raw: *const c_int,
    owner: PhantomData<&'c Catalog>,
}

impl Catalog {
    fn first(&self) -> PriceRef<'_> {
        PriceRef { raw: self.prices.as_ptr(), owner: PhantomData }
    }
}

impl PriceRef<'_> {
    fn get(&self) -> c_int {
        // SAFETY: the catalog is borrowed for as long as this value exists.
        unsafe { *self.raw }
    }
}

fn main() {
    let price = {
        let catalog = Catalog { prices: vec![250, 990] };
        catalog.first()
    };
    println!("{}", price.get());
}
```

```text
error[E0597]: `catalog` does not live long enough
  --> src/main.rs:29:9
   |
27 |     let price = {
   |         ----- borrow later stored here
28 |         let catalog = Catalog { prices: vec![250, 990] };
   |             ------- binding `catalog` declared here
29 |         catalog.first()
   |         ^^^^^^^ borrowed value does not live long enough
30 |     };
   |     - `catalog` dropped here while still borrowed
```

Without the `PhantomData` field the same program compiles, and then it reads
freed memory. Fix in the calling code: keep the owner alive, or copy the
value out (`catalog.first().get()`) before the owner goes away. Changing the
owner is rejected in the same way: a `catalog.prices.push(120)` between
`first()` and `get()` gives `E0502`, because a push may move the buffer.

**Declaring the lifetime but not using it.**

```rust,compile_fail,E0392
use std::ffi::c_int;

struct PriceRef<'c> {
    raw: *const c_int,
}

fn main() {}
```

```text
error[E0392]: lifetime parameter `'c` is never used
 --> src/main.rs:3:17
  |
3 | struct PriceRef<'c> {
  |                 ^^ unused lifetime parameter
  |
  = help: consider removing `'c`, referring to it in a field, or using a marker such as `PhantomData`
```

A raw pointer does not use a lifetime. The `help` line names the fix.

**Letting the compiler choose the lifetime of a reference made from a raw
pointer.** `CStr::from_ptr(p)` and `&*p` produce a reference whose lifetime
is whatever the surrounding code asks for, even `'static`. The compiler
accepts any choice, because a raw pointer gives it nothing to check against.
So never return such a reference from a function whose signature does not tie
it to an owner. Write the signature first (`fn name(&self) -> &CStr`), and
let it limit the lifetime.

### Read more

- [`PhantomData`, in the standard library](https://doc.rust-lang.org/std/marker/struct.PhantomData.html)
- [PhantomData, in the Rustonomicon](https://doc.rust-lang.org/nomicon/phantom-data.html)
- [Unbounded lifetimes, in the Rustonomicon](https://doc.rust-lang.org/nomicon/unbounded-lifetimes.html)

The mastery module [`mastery/m1_types`](../../mastery/m1_types/README.md) goes deeper into `PhantomData`,
variance and the drop check.

## Lesson 08 · Calling the real C library

Do this lesson after Lesson 06 and before the capstone. It assumes C strings
(Lesson 02), `unsafe extern "C" fn` types (Lesson 03) and callbacks
(Lesson 04).

### The idea

Until now, every "foreign" function had a Rust body in the same file. A real
foreign function has no body that Rust can see. It lives in a compiled
library. To call it, you write a **declaration** in an `unsafe extern` block:
the name, the parameter types and the return type, and no body.

Nothing checks that declaration. Rust cannot read C header files. The
**linker**, the program that joins compiled pieces into one executable,
matches a call to a function by name only. It knows nothing about types. If
your declaration says `c_int` where the C function takes a `size_t`, the
program still builds, and then it misbehaves. That is why the block itself
is `unsafe`: with that word you promise that every declaration in it matches
the real function.

Every function in the block is unsafe to call, unless you mark it `safe fn`.
Do that only when no argument at all can cause undefined behavior.

The C standard library is already linked into every Rust program that uses
`std`, on Linux, macOS and Windows. So its functions are a real foreign
library that needs no C compiler and no build script.

### Worked example

Two C functions, translated from their prototypes. `strcmp` compares two
strings. `bsearch` searches a sorted array, and asks a callback to compare
the key with an element.

```rust
use std::cmp::Ordering;
use std::ffi::{CStr, c_char, c_int, c_void};

unsafe extern "C" {
    // int strcmp(const char *left, const char *right);
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;

    // void *bsearch(const void *key, const void *base, size_t nmemb,
    //               size_t size, int (*compar)(const void *, const void *));
    fn bsearch(
        key: *const c_void,
        base: *const c_void,
        nmemb: usize,
        size: usize,
        compar: unsafe extern "C" fn(*const c_void, *const c_void) -> c_int,
    ) -> *mut c_void;
}

/// Orders two C strings as C does: byte by byte.
fn c_order(left: &CStr, right: &CStr) -> Ordering {
    // SAFETY: both arguments are live, NUL-terminated strings, because that
    // is what a CStr is. strcmp only reads them and keeps no pointer.
    let sign = unsafe { strcmp(left.as_ptr(), right.as_ptr()) };
    // C promises only the sign of the result, not -1 and 1.
    sign.cmp(&0)
}

/// # Safety
/// Both pointers refer to live, aligned u16 values.
unsafe extern "C" fn compare_u16(key: *const c_void, element: *const c_void) -> c_int {
    // SAFETY: promised by the caller.
    let (key, element) = unsafe { (*key.cast::<u16>(), *element.cast::<u16>()) };
    // Two u16 values are at most 65_535 apart, so after widening to c_int
    // this subtraction cannot overflow, and so it cannot panic.
    c_int::from(key) - c_int::from(element)
}

/// Finds `port` in a sorted slice and returns its index.
fn find_port(sorted: &[u16], port: u16) -> Option<usize> {
    // SAFETY: base points to sorted.len() elements of size_of::<u16>() bytes
    // each, the key is a live u16, and compare_u16 reads both arguments as
    // u16. bsearch only reads, and keeps no pointer after it returns.
    let found = unsafe {
        bsearch(
            (&raw const port).cast(),
            sorted.as_ptr().cast(),
            sorted.len(),
            size_of::<u16>(),
            compare_u16,
        )
    };
    if found.is_null() {
        return None; // C says "not found" with a null pointer
    }
    // SAFETY: a result that is not null points at an element of `sorted`.
    let offset = unsafe { found.cast::<u16>().offset_from(sorted.as_ptr()) };
    usize::try_from(offset).ok()
}

assert_eq!(c_order(c"apple", c"apricot"), Ordering::Less);
assert_eq!(c_order(c"pear", c"pear"), Ordering::Equal);
let ports = [22, 80, 443, 8080];
assert_eq!(find_port(&ports, 443), Some(2));
assert_eq!(find_port(&ports, 25), None);
```

Read the translation type by type: `const char *` becomes `*const c_char`,
`size_t` becomes `usize`, `void *` becomes `*mut c_void`, and a C function
pointer becomes an `unsafe extern "C" fn` type. Note the two numbers that
describe the array: the **count** of elements, and the size of **one**
element in bytes. C cannot see the element type behind a `void *`, so it
needs both.

### What the compiler can still notice

The compiler knows the signatures of a few C functions that the standard
library itself uses. For those it warns:

```rust
use std::ffi::{c_int, c_void};

unsafe extern "C" {
    // The real prototype is:
    // int memcmp(const void *left, const void *right, size_t n);
    fn memcmp(left: *const c_void, right: *const c_void, n: c_int) -> c_int;
}

fn main() {}
```

```text
warning: suspicious definition of the runtime `memcmp` symbol used by the standard library
 --> src/main.rs:6:5
  |
6 |     fn memcmp(left: *const c_void, right: *const c_void, n: c_int) -> c_int;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: expected `unsafe extern "C" fn(*const c_void, *const c_void, usize) -> i32` (for the current target)
          found    `unsafe extern "C" fn(*const c_void, *const c_void, i32) -> i32`
```

It also warns when one crate declares the same foreign function twice with
different types:

```rust
mod screen {
    use std::ffi::c_int;
    unsafe extern "C" {
        pub fn toupper(ch: c_int) -> c_int;
    }
}

mod keyboard {
    unsafe extern "C" {
        pub fn toupper(ch: u8) -> u8;
    }
}

fn main() {}
```

```text
warning: `toupper` redeclared with a different signature
  --> src/main.rs:10:9
   |
 4 |         pub fn toupper(ch: c_int) -> c_int;
   |         ----------------------------------- `toupper` previously declared here
...
10 |         pub fn toupper(ch: u8) -> u8;
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ this signature doesn't match the previous declaration
   |
   = note: expected `unsafe extern "C" fn(i32) -> i32`
              found `unsafe extern "C" fn(u8) -> u8`
   = note: `#[warn(clashing_extern_declarations)]` on by default
```

The exercise uses this second lint as a safety net. The course keeps a
correct declaration of each function in a support file, and turns the lint
into an error for the block that you write. In your own projects there is no
such net: a single wrong declaration of a function that the compiler does
not know gives no message at all.

### Mistakes people make

**Declaring a function that does not exist, or misspelling its name.** The
compiler accepts the declaration, and it accepts a call to the function too.
The linker then fails, because no library contains that name. On macOS the
message ends like this (other platforms use other words):

```text
error: linking with `cc` failed: exit status: 1
  ...
  = note: Undefined symbols for architecture arm64:
            "_frobnicate_widget", referenced from:
  ...
          ld: symbol(s) not found for architecture arm64
```

**Marking a function `safe` because it "only takes a number".** C's `toupper`
has undefined behavior for most negative arguments. With
`safe fn toupper(ch: c_int) -> c_int;` in the block, `toupper(-5000)`
compiles without `unsafe`, and the mistake is yours. Leave such a function
unsafe, and write a safe Rust wrapper whose parameter type (here `u8`) allows
only good values.

**A comparator that can panic.** The classic comparator returns `a - b`. For
full-width integers that subtraction can overflow. In a debug build an
overflow is a panic, and a panic inside an `extern "C"` function aborts the
process, as you saw in Lesson 04. In a release build the result wraps
around and has the wrong sign. C then promises nothing about the result of
the sort; usually the array comes out in the wrong order. Compare with `<` and `>`,
or with `cmp`, and never with subtraction, unless you can show that the
difference always fits (as `compare_u16` does above).

**Passing the size of the whole array where C wants the element count**, or
the other way round. C then reads or writes far outside your buffer. Take
both numbers from the slice and its element type, never from literals.

### Beyond this lesson

For a library that is not already linked, you also need a `#[link]`
attribute or a build script, and for a large header you generate the
declarations with a tool instead of writing them by hand. The optional labs
in [LABS.md](LABS.md) give you a first try. The mastery module
[`mastery/m6_ffi`](../../mastery/m6_ffi/README.md) builds a real C library, generates bindings and exports Rust
functions to C.

### Read more

- [External blocks, in the Rust Reference](https://doc.rust-lang.org/reference/items/external-blocks.html)
- [`unsafe extern` blocks, in the Edition Guide](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-extern.html)
- [Foreign function interface, in the Rustonomicon](https://doc.rust-lang.org/nomicon/ffi.html)
- [The `clashing_extern_declarations` lint](https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#clashing-extern-declarations)

## Lesson 07 · Capstone: checked foreign transaction

Do the capstone after Lesson 08 and the drills. It uses Lessons 02, 03 and 06
together, on one foreign function that has two out-parameters.

### The approach

Do not start with the code. Start with a table of everything that crosses
the boundary. For each pointer, answer four questions: who owns the memory,
when is it written, when may it be read, and until when is it valid? For the
capstone, your table has one row for the name, one for the result slot and
one for the error slot. Fill it in from the comment above the foreign
function. Your `// SAFETY:` comments are then the rows of the table.

Next, walk through the call in order, and decide for each step what happens
when it fails:

1. **Arguments.** Turn Rust values into C values. Anything that cannot be
   represented is an error, before the library is called.
2. **Storage.** Make room for everything that the function may write. Use
   the type that tells the truth about its state.
3. **The call.** One `unsafe` block, as small as possible.
4. **The status.** It decides which slot holds a value. Read that slot, and
   only that one.
5. **Borrowed results.** A pointer that the library lends you can change on
   the next call. Copy what you need at once, into a value that Rust owns.

A reader of your function should be able to see these five steps.

### The second task: many calls

The batch function repeats the call. Two questions matter. What does "stop"
mean for a foreign library? Calls that you did not make cannot disturb any
state inside it. And how does the caller learn **which** input failed? Look
again at how iterators collect into a `Result` (Chapter 15) and at
`enumerate` before you write a loop by hand.

### Check your work like a reviewer

When the tests pass, read your own code as if it were a pull request. Is
there an `unwrap` that turns bad input into a panic? Is there a path on
which uninitialized memory is read? Does any reference live longer than the
buffer that it points into? Could any step panic while C code is on the call
stack? The code review lesson (C01) has you ask the same questions about
someone else's wrapper.

### Read more

- [Creating a safe interface, in the Rustonomicon](https://doc.rust-lang.org/nomicon/ffi.html#creating-a-safe-interface)
- [Working with unsafe, in the Rustonomicon](https://doc.rust-lang.org/nomicon/working-with-unsafe.html)
