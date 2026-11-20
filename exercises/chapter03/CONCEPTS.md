# Chapter 3 concepts — Numbers, text and arrays

[Chapter guide](README.md) · [Hints](HINTS.md) · [Your notes](NOTES.md)

Read one section, then do the lesson with the same number. Each section gives you the idea, one worked example, the mistakes people really make, and links to the official documentation.

## What you will be able to do

Every value in a Rust program has a type, and the type decides three things: how many bytes the value takes, which operations are allowed, and what happens at the edges (a number that is too big, text that is not ASCII, an index past the end). This chapter covers the types that everything else is built from.

**After this chapter you can:**

- read and write numbers in any base, and say what a cast will do before you run it;
- choose what should happen when arithmetic overflows, instead of leaving it to the build profile;
- work with floating-point values without being surprised by rounding, NaN or signed zero;
- pick between an array, a vector and a slice, and between `String` and `&str`;
- tell bytes, Unicode scalar values and user-perceived characters apart;
- convert between numeric types, bytes and text with checks where a value might not fit.

## Before you start

Chapters 1 and 2 of this course: you can run `cargo test` for one lesson, write a function, use `let` and `mut`, write `if`, `for` and `while`, and read a failing test. Nothing else is assumed.

Two standard-library types appear here before their own chapters, because many numeric and text methods return them:

- `Option<T>` is either `Some(value)` or `None`. It means "a value, or nothing".
- `Result<T, E>` is either `Ok(value)` or `Err(error)`. It means "a value, or the reason there is none".

In this chapter you only build them, compare them, and look inside with `match`. Chapter 7 covers them fully.

## Time

These are estimates for a first pass, including predictions, tests and notes. Your times will differ.

| Lesson | Estimate |
| --- | --- |
| 01 Inference | 30 min |
| 02 Integers | 35 min |
| 03 Overflow | 50 min |
| 04 Floating point | 40 min |
| 05 Booleans, tuples, unit | 30 min |
| 06 Pointer types | 50 min |
| 07 Arrays | 30 min |
| 08 Vectors | 40 min |
| 09 Slices | 40 min |
| 10 Unicode | 50 min |
| 11 Strings | 45 min |
| 12 Boundary types | 35 min |
| 14 Checked conversions | 60 min |
| 13 Capstone | 60 min |
| C01 Code review | 45 min |
| Compiler drills | 40 min |

That is about 11 hours in total. Plan several sittings.

## About the code blocks

Every Rust block in this file is compiled and tested with the pinned toolchain (Rust 1.99.0, edition 2024). Blocks that are meant to fail are followed by the real compiler or panic output, trimmed to the important lines. In panic messages, the number in parentheses after the thread name is a thread id; yours will be different.

## Lesson 01 · Inference and numeric method lookup

**Assumed:** functions and `let` from chapters 1 and 2.

Rust is statically typed: every value has exactly one type, and the compiler knows it before the program runs. You do not have to write most of those types. You must write them in a function's signature (its parameter types and its return type), and on a few other items that later chapters introduce, such as constants. Inside a function body, the compiler works out the rest. This is called *type inference*.

Inference collects clues. A clue can be an annotation (`let n: u8 = 7;`), a suffix on a literal (`7_u8`), the function signature, or the way a value is used. A clue may come from a line *after* the one that creates the value.

A number literal without a suffix starts with a placeholder type. The compiler prints these placeholders as `{integer}` and `{float}`. If some clue fixes the type, the literal takes that type. If no clue exists anywhere, the compiler falls back to `i32` for integers and `f64` for floats.

### Worked example

```rust
fn total_millis(seconds: u64) -> u64 {
    let per_second = 1_000; // no suffix: u64, because of the next line
    seconds * per_second
}

fn main() {
    assert_eq!(total_millis(3), 3_000);

    let mut names = Vec::new(); // the element type is not known yet
    names.push("ada"); // now it is: this is a Vec<&str>
    assert_eq!(names.len(), 1);

    // Three ways to give a clue yourself.
    let a: u8 = 7; // annotation on the binding
    let b = 7_u8; // suffix on the literal
    let c = Vec::<u8>::new(); // type argument written in the call
    assert_eq!(a, b);
    assert!(c.is_empty());
}
```

In `total_millis`, nothing on the `let` line says what `per_second` is. The multiplication does: `u64 * something` only works when the other side is `u64` too.

### Calling a method on a literal

A method call is different from arithmetic. To find the method, the compiler needs the *concrete* type of the value at that point. A placeholder is not enough, and the `i32`/`f64` fallback is applied too late to help.

```rust,compile_fail,E0689
fn main() {
    let side = 3.0;
    let area = side.powi(2);
    println!("{area}");
}
```

```text
error[E0689]: can't call method `powi` on ambiguous numeric type `{float}`
 --> src/main.rs:3:21
  |
3 |     let area = side.powi(2);
  |                     ^^^^
  |
help: you must specify a type for this binding, like `f32`
  |
2 |     let side: f32 = 3.0;
  |             +++++
```

Fix it with any clue: `let side: f64 = 3.0;`, or `3.0_f64`, or name the type in the call: `f64::powi(side, 2)`.

### Two more mistakes

**No clue at all.** If nothing ever uses the vector's elements, the compiler cannot guess their type.

```rust,compile_fail,E0282
fn main() {
    let readings = Vec::new();
    println!("{}", readings.len());
}
```

```text
error[E0282]: type annotations needed for `Vec<_>`
 --> src/main.rs:2:9
  |
2 |     let readings = Vec::new();
  |         ^^^^^^^^   ---------- type must be known at this point
```

Fix: `let readings: Vec<f64> = Vec::new();`.

**Unary minus and method calls.** A method call binds tighter than the minus sign in front of the number. So the minus is applied *after* the method.

```rust
fn main() {
    assert_eq!(-2_i32.pow(2), -4); // read as -(2.pow(2))
    assert_eq!((-2_i32).pow(2), 4); // the parentheses make -2 the receiver
}
```

The predictions of this lesson also use `leading_zeros()`, which lesson 02 explains in detail. For now you need only this: it counts the 0 bits in front of the highest 1 bit, so its answer depends on how many bits the type has. `1_u16.leading_zeros()` is 15, and `1_u64.leading_zeros()` is 63. That makes it a good probe for the question "which type did inference pick?".

### When to use which

- Annotate the binding (`let n: u64 = ...`) when the type matters to the reader.
- Use a suffix (`0_u8`) for a single literal inside an expression.
- Use a type argument (`Vec::<u8>::new()`, `"5".parse::<u8>()`) when a generic function is called in the middle of an expression and there is no binding to annotate.
- Otherwise write nothing and let the signature do the work.

### Read more

- [Integer literal expressions](https://doc.rust-lang.org/reference/expressions/literal-expr.html#integer-literal-expressions) in the Rust Reference (suffixes and the `i32` fallback)
- [Expression precedence](https://doc.rust-lang.org/reference/expressions.html#expression-precedence) in the Rust Reference
- [`let` statements](https://doc.rust-lang.org/reference/statements.html#let-statements) in the Rust Reference

## Lesson 02 · Integer representations, literals, and casts

**Assumed:** lesson 01.

An integer type is a fixed number of bits. The name tells you how many and whether negative values are allowed: `u8`, `u16`, `u32`, `u64`, `u128` are unsigned, and `i8` to `i128` are signed. With N bits:

- an unsigned type holds 0 to 2^N − 1 (for `u8`: 0 to 255);
- a signed type holds −2^(N−1) to 2^(N−1) − 1 (for `i8`: −128 to 127).

Signed types use *two's complement*: the top bit counts as −2^(N−1) instead of +2^(N−1). So the same eight bits can mean two different numbers, depending on the type you read them with.

```text
bits         read as u8   read as i8
0000_0101          5            5
0111_1111        127          127
1000_0000        128         -128
1111_1101        253           -3
```

`usize` and `isize` are as wide as a memory address on the target (64 bits on the platforms this course supports). Lengths and indexes are `usize`, because a collection can be as large as memory allows.

### Literals

You can write the same number in several ways. `_` is only a visual separator.

```rust
fn main() {
    assert_eq!(1_000_000, 1000000);
    assert_eq!(0xff, 255); // hexadecimal
    assert_eq!(0o17, 15); // octal
    assert_eq!(0b1111_0000, 0xf0); // binary
    assert_eq!(b'A', 65_u8); // byte literal: the ASCII code, as a u8
    assert_eq!(u8::MAX, 255);
    assert_eq!(i8::MIN, -128);
}
```

### Casts with `as`

Rust never converts between integer types by itself. You ask with `as`, and an integer-to-integer `as` cast never fails and never panics. It follows three rules:

- **To a narrower type:** keep the low bits, drop the rest.
- **Same width, signed to unsigned or back:** keep all the bits, change how they are read.
- **To a wider type:** the value is kept whenever the target can hold it. A signed source is *sign-extended*: the new high bits are copies of its top bit, so a negative number stays negative in a wider signed type. If the wider target is unsigned, those same bits are then read as a large positive number.

### Worked example

A pixel is stored as one `u32` in the form `0xRRGGBB`. To take out the green part, shift it to the bottom, mask it, then narrow it.

```rust
fn main() {
    let pixel: u32 = 0x33_cc_99;
    let green = ((pixel >> 8) & 0xff) as u8;
    assert_eq!(green, 0xcc);

    let delta: i8 = -3;
    assert_eq!(format!("{delta:08b}"), "11111101");
    assert_eq!(delta as u8, 253); // same bits, read as unsigned
    assert_eq!(delta as i16, -3); // wider: the value is kept
    assert_eq!(1_000_i32 as u8, 232); // 1000 is 0x3e8; the low byte is 0xe8
}
```

The last line is the dangerous one. Nothing warns you that 1000 became 232. Use `as` to narrow only when dropping the high bits is what you want.

### Looking at the bits

Every integer type has methods that inspect the bit pattern, so you do not need a loop. Each of them returns a `u32`. The constant `BITS` is the width of the type.

```rust
fn main() {
    let flags: u8 = 0b0010_0110;
    assert_eq!(flags.count_ones(), 3); // how many bits are 1
    assert_eq!(flags.count_zeros(), 5);
    assert_eq!(flags.leading_zeros(), 2); // zeros before the highest 1
    assert_eq!(flags.trailing_zeros(), 1); // zeros after the lowest 1
    assert_eq!(u8::BITS, 8);
    assert_eq!(size_of::<u8>(), 1); // size_of counts bytes, not bits
}
```

### Mistakes people make

**Using a non-`usize` index.** Indexes are `usize`; no other integer type is accepted.

```rust,compile_fail,E0277
fn main() {
    let levels = [10, 20, 30];
    let position: u32 = 1;
    println!("{}", levels[position]);
}
```

```text
error[E0277]: the type `[{integer}]` cannot be indexed by `u32`
   --> src/main.rs:4:27
    |
  4 |     println!("{}", levels[position]);
    |                           ^^^^^^^^ slice indices are of type `usize` or ranges of `usize`
```

Fix: declare `position` as `usize`. If the value arrives as a `u32`, convert it once, where it enters your code.

**A literal that does not fit.** The compiler checks literals against the type.

```rust,compile_fail
fn main() {
    let big: u8 = 256;
    println!("{big}");
}
```

```text
error: literal out of range for `u8`
 --> src/main.rs:2:19
  |
2 |     let big: u8 = 256;
  |                   ^^^
  |
  = note: the literal `256` does not fit into the type `u8` whose range is `0..=255`
  = note: `#[deny(overflowing_literals)]` on by default
```

This check only covers literals. A value computed at run time gets no such check; that is the subject of lesson 03.

### When to use which

- `u32::from(x)` when the conversion can never lose anything. It only compiles for such pairs.
- `u8::try_from(x)` when the value might not fit and you want to know (lesson 14).
- `x as u8` when you want the low bits or a different reading of the same bits.

For the type itself: `usize` for lengths and indexes, `u8` for raw bytes, `i32` when you have no reason to choose something else, `u64` or `i64` for counters, sizes and timestamps.

### Read more

- [`u8`](https://doc.rust-lang.org/std/primitive.u8.html) and [`i32`](https://doc.rust-lang.org/std/primitive.i32.html) in the standard library
- [Numeric cast semantics](https://doc.rust-lang.org/reference/expressions/operator-expr.html#numeric-cast) in the Rust Reference
- [Integer types](https://doc.rust-lang.org/reference/types/numeric.html#integer-types) in the Rust Reference

## Lesson 03 · Choosing an overflow policy

**Assumed:** lessons 01 and 02; `Option` from the overview.

An integer type has a fixed width, so the true result of an operation may not fit. `200_u8 + 100` should be 300, and a `u8` stops at 255. That is *overflow*.

With the plain operators (`+`, `-`, `*`), what happens depends on how you built the program:

- a debug build (`cargo build`, `cargo test`) panics;
- a release build (`--release`) keeps the low bits and continues without a word.

So plain `+` means "I am sure this cannot overflow". When overflow *can* happen, choose the behaviour yourself. Every integer type has the same families of methods:

| Family | On overflow | Typical use |
| --- | --- | --- |
| `checked_add`, `checked_mul`, ... | returns `None` | a size or amount that comes from input |
| `wrapping_add`, ... | keeps the low bits (wraps around) | hashes, sequence numbers |
| `saturating_add`, ... | stops at `MIN` or `MAX` | a volume level, an audio sample |
| `overflowing_add`, ... | returns `(wrapped value, true)` | arithmetic on numbers wider than one integer |
| `strict_add`, ... | panics in every build | "this must never happen" |

All of these behave the same in debug and release builds. That is the point.

### Worked example

How many bytes does an image need, at four bytes per pixel? Width and height come from a file, so the product may not fit.

```rust
/// Bytes for a width x height image, or None if the size does not fit in u32.
fn image_bytes(width: u32, height: u32) -> Option<u32> {
    match width.checked_mul(height) {
        Some(pixels) => pixels.checked_mul(4),
        None => None,
    }
}

fn main() {
    assert_eq!(image_bytes(640, 480), Some(1_228_800));
    assert_eq!(image_bytes(70_000, 70_000), None);

    let volume: u8 = 250;
    assert_eq!(volume.saturating_add(20), 255); // stops at the top
    assert_eq!(5_u8.saturating_sub(20), 0); // and at the bottom

    let sequence: u8 = 255;
    assert_eq!(sequence.wrapping_add(1), 0); // goes around
    assert_eq!(sequence.overflowing_add(3), (2, true));
    assert_eq!(i8::MAX.checked_add(1), None);
}
```

### Shifts

`value << distance` has its own kind of overflow: a distance that is as large as the type's width or larger. The policy methods for shifts look only at the *distance*. Bits pushed out of the top are lost without any report, because that is what a shift does.

```rust
fn main() {
    assert_eq!(0b1000_0001_u8.checked_shl(1), Some(0b0000_0010)); // top bit lost: still Some
    assert_eq!(1_u8.checked_shl(8), None); // distance 8 on an 8-bit type
    assert_eq!(1_u8.wrapping_shl(9), 2); // the distance wraps: 9 becomes 1
}
```

### Mistakes people make

**Trusting plain `+` with run-time values.**

```rust,should_panic
fn next_level(level: u8) -> u8 {
    level + 1
}

fn main() {
    println!("{}", next_level(255));
}
```

```text
thread 'main' (4521) panicked at src/main.rs:2:5:
attempt to add with overflow
```

Built with `--release`, the same program prints `0`. When the compiler can see both values at compile time, it refuses to build at all (`error: this arithmetic operation will overflow`), but you cannot rely on that for real inputs.

**Widening after the operation.** The cast comes too late: the overflow already happened in the narrow type.

```rust
fn main() {
    let (a, b): (u8, u8) = (200, 100);
    assert_eq!(a.wrapping_add(b) as u16, 44); // what `(a + b) as u16` gives in release
    assert_eq!(u16::from(a) + u16::from(b), 300); // widen first, then add
}
```

**Expecting `checked_shl` to detect lost bits.** It checks the distance only, as shown above. To check a multiplication by a power of two, use `checked_mul`.

### When to use which

Ask what the *caller* needs when the result does not fit. An error: `checked_`. A value that stays at the limit: `saturating_`. A value that goes around on purpose: `wrapping_`. A crash, because continuing would be wrong: `strict_`. Use plain operators only when the ranges of the inputs make overflow impossible.

### Read more

- [`checked_add`](https://doc.rust-lang.org/std/primitive.u8.html#method.checked_add), [`wrapping_add`](https://doc.rust-lang.org/std/primitive.u8.html#method.wrapping_add), [`saturating_add`](https://doc.rust-lang.org/std/primitive.u8.html#method.saturating_add) and [`overflowing_add`](https://doc.rust-lang.org/std/primitive.u8.html#method.overflowing_add) on `u8`
- [Overflow](https://doc.rust-lang.org/reference/expressions/operator-expr.html#overflow) in the Rust Reference
- [`overflow-checks`](https://doc.rust-lang.org/cargo/reference/profiles.html#overflow-checks) in the Cargo book (the profile setting behind the debug/release difference)

## Lesson 04 · Floating-point values and explicit conversions

**Assumed:** lessons 01 to 03.

`f32` and `f64` store numbers in binary scientific notation: a sign, a fixed number of significant bits, and an exponent. `f64` has 53 significant bits (about 15 to 17 decimal digits). `f32` has 24 (about 7 digits). Use `f64` unless you have a reason not to.

Two consequences follow.

**Most decimal fractions are not exact.** 0.1 has no finite binary form, in the same way that 1/3 has no finite decimal form. The stored value is the nearest one that fits, and every operation rounds its result again. Two calculations that are equal on paper can differ in the last bit.

**There are special values.** Dividing a non-zero number by zero gives positive or negative *infinity*. An operation with no meaningful answer, such as the square root of a negative number, gives *NaN* ("not a number"). NaN is not equal to anything, including itself, and every `<` or `>` comparison with NaN is false. There are also two zeros, `0.0` and `-0.0`. They compare as equal, but they carry different signs.

Rust never mixes integers and floats for you. Convert explicitly: `f64::from(n)` when the conversion is exact (for example from `i16`, `i32`, `u32` or `f32`), and `n as f64` otherwise.

Mathematical constants are in the module `std::f64::consts` (and `std::f32::consts`). Write `std::f64::consts::PI` instead of typing the digits yourself: the constant is the closest `f64` to π.

### Worked example

```rust
fn average(total: f64, count: u32) -> f64 {
    total / f64::from(count)
}

/// Equal within a small tolerance.
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn main() {
    assert_eq!(average(7.0, 2), 3.5);

    let circumference = 2.0 * std::f64::consts::PI * 0.5; // radius 0.5
    assert!(close(circumference, 3.141592653589793));

    let sum = 0.1 + 0.7;
    assert!(sum != 0.8); // rounding: the sum is not exactly 0.8
    assert_eq!(format!("{sum}"), "0.7999999999999999");
    assert!(close(sum, 0.8)); // compare with a tolerance instead

    let nothing = f64::sqrt(-1.0);
    assert!(nothing.is_nan());
    assert!(!(nothing < 1.0) && !(nothing >= 1.0)); // every comparison is false
    assert!((f64::MAX * 2.0).is_infinite());

    // Four ways to get a whole number.
    assert_eq!((-2.7_f64).floor(), -3.0); // down
    assert_eq!((-2.7_f64).ceil(), -2.0); // up
    assert_eq!((-2.7_f64).trunc(), -2.0); // toward zero
    assert_eq!((-2.7_f64).round(), -3.0); // nearest
    assert_eq!(0.5_f64.round(), 1.0); // a half rounds away from zero

    // Widening an f32 is exact, but it cannot bring back lost digits.
    let third = 1.0_f32 / 3.0;
    assert!(f64::from(third) != 1.0_f64 / 3.0);
}
```

The last two lines matter for mixed code. An `f32` holds about 7 digits. Converting it to `f64` afterwards gives you those same 7 digits followed by noise. If you need `f64` precision, convert the inputs *before* you calculate. For whole numbers, `f32` is exact only up to 2^24 (16,777,216); above that it starts to skip values.

### Mistakes people make

**Mixing an integer with a float.**

```rust,compile_fail,E0277
fn main() {
    let price = 2.5;
    let count = 3;
    let total = price * count;
    println!("{total}");
}
```

```text
error[E0277]: cannot multiply `{float}` by `{integer}`
 --> src/main.rs:4:23
  |
4 |     let total = price * count;
  |                       ^ no implementation for `{float} * {integer}`
```

Fix: `price * f64::from(count)`. The conversion is visible, so a reader can see where precision could change.

**Comparing computed floats with `==`.** As the example shows, `0.1 + 0.7 == 0.8` is false. Compare with a tolerance that suits your data. `==` is fine for values that were stored and not computed, and for values that are exact in binary such as `0.5` or `2.0`.

**Testing for NaN with `==`.** `x == f64::NAN` is always false. Use `x.is_nan()`. Remember also that `!(x < limit)` and `x >= limit` are different conditions when `x` can be NaN.

### When to use which

- `f64` by default; `f32` when you store millions of values and 7 digits are enough.
- An integer count of the smallest unit (cents, milliseconds) for money and for anything that must add up exactly.
- `is_finite()` to reject both infinities and NaN before you use an input.

### Read more

- [`f64`](https://doc.rust-lang.org/std/primitive.f64.html) in the standard library (see `is_nan`, `round`, `EPSILON`)
- [Floating-point types](https://doc.rust-lang.org/reference/types/numeric.html#floating-point-types) in the Rust Reference
- [`std::f64::consts`](https://doc.rust-lang.org/std/f64/consts/index.html) for constants such as π

## Lesson 05 · Explicit conditions, tuples, and unit

**Assumed:** lessons 01 to 04.

### `bool`

A `bool` is `true` or `false`. `if` and `while` accept only a `bool`. A number, a string or a pointer is never treated as true or false; you write the comparison you mean.

You can cast a `bool` to an integer: `false` becomes 0 and `true` becomes 1. The reverse cast does not exist.

### Tuples

A tuple groups a fixed number of values that may have different types. The type lists each field: `(u32, f64, &str)`. You reach a field by position with `.0`, `.1`, and so on, or you take the tuple apart with a pattern: `let (a, b) = pair;`. Tuples are the simple way for a function to return more than one value. Standard-library methods do this too. You met `overflowing_add` in lesson 03. Another is `split_at` on text: `"id=42".split_at(2)` cuts the text at an offset and returns both parts as a pair, `("id", "=42")`. Each part is a `&str`, a borrowed view of the original text, and the offset counts bytes; lessons 09 and 10 explain both ideas.

A tuple is stored inline as one value, field next to field. Each field must start at an address that suits its type, so the compiler may leave unused *padding* bytes between fields. A `(u8, u16)` takes 4 bytes, not 3.

Two details of syntax:

- `(7,)` is a tuple with one field. `(7)` is only the number 7 in parentheses.
- Field access needs a literal position. `pair.0` works; `pair[i]` does not exist, because the fields can have different types and the compiler could not say what type `pair[i]` has.

Tuples whose fields can be compared can be compared too, with `==` and `<`. The comparison goes field by field from the left and stops at the first difference.

### Unit

`()` is the tuple with no fields. Its type is also written `()` and is called *unit*. It has exactly one value and carries no information. A function without `->` returns `()`. So does a block whose last line ends with a semicolon.

### Worked example

```rust
/// Quotient and remainder in one call.
fn divide(dividend: u32, divisor: u32) -> (u32, u32) {
    (dividend / divisor, dividend % divisor)
}

fn log_line(message: &str) {
    println!("log: {message}");
} // no `->`, so this returns ()

fn main() {
    let (quotient, remainder) = divide(17, 5);
    assert_eq!((quotient, remainder), (3, 2));

    let pair = divide(9, 4);
    assert_eq!(pair.0, 2);
    assert_eq!(pair.1, 1);

    let (key, rest) = "id=42".split_at(2); // a method that returns a pair
    assert_eq!((key, rest), ("id", "=42"));

    let late = true;
    let fee = 10 + 5 * u32::from(late); // `late as u32` does the same
    assert_eq!(fee, 15);

    let single = (7,);
    assert_eq!(single.0, 7);
    assert!(("apple", 3) < ("banana", 1)); // decided by the first field
    assert_eq!(size_of::<(u8, u16)>(), 4);

    let (mut low, mut high) = (9, 2);
    (low, high) = (high, low); // the right side is built first, then assigned
    assert_eq!((low, high), (2, 9));

    let nothing: () = log_line("ready");
    assert_eq!(nothing, ());
}
```

### Mistakes people make

**A number as a condition.**

```rust,compile_fail,E0308
fn main() {
    let mut retries = 3;
    while retries {
        retries -= 1;
    }
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:3:11
  |
3 |     while retries {
  |           ^^^^^^^ expected `bool`, found integer
```

Fix: `while retries > 0`.

**A semicolon after the value you wanted to return.** The semicolon turns the last expression into a statement, so the body's value is `()`.

```rust,compile_fail,E0308
fn double(n: i32) -> i32 {
    n * 2;
}

fn main() {
    println!("{}", double(4));
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:1:22
  |
1 | fn double(n: i32) -> i32 {
  |    ------            ^^^ expected `i32`, found `()`
  |    |
  |    implicitly returns `()` as its body has no tail or `return` expression
2 |     n * 2;
  |          - help: remove this semicolon to return this value
```

**Casting an integer to `bool`.**

```rust,compile_fail,E0054
fn main() {
    let flag = 1_u8 as bool;
    println!("{flag}");
}
```

```text
error[E0054]: cannot cast `u8` as `bool`
 --> src/main.rs:2:16
  |
2 |     let flag = 1_u8 as bool;
  |                ^^^^^^^^^^^^
  |
help: compare with zero instead
  |
2 -     let flag = 1_u8 as bool;
2 +     let flag = 1_u8 != 0;
  |
```

### When to use which

Use a tuple for two or three values whose meaning is obvious from their position, such as a quotient and a remainder. When a reader would have to ask "which one is `.2`?", the values need names. That is a struct (chapter 9).

### Read more

- [`bool`](https://doc.rust-lang.org/std/primitive.bool.html), [tuples](https://doc.rust-lang.org/std/primitive.tuple.html) and [unit](https://doc.rust-lang.org/std/primitive.unit.html) in the standard library
- [Tuple types](https://doc.rust-lang.org/reference/types/tuple.html) in the Rust Reference
- [Block expressions](https://doc.rust-lang.org/reference/expressions/block-expr.html) in the Rust Reference (why a trailing semicolon changes the value)

## Lesson 06 · Pointer types encode different policies

**Assumed:** lessons 01 to 05. This lesson is a survey. Chapters 4 and 5 study ownership and borrowing in depth.

A *pointer* is a value that holds the address of another value. Rust has several pointer types. They all hold an address; they differ in the answers to two questions:

1. Who frees the value that the pointer leads to?
2. Who may change it?

First, one word. The *owner* of a value is the variable responsible for it. When the owner goes out of scope, Rust frees the value. This is called *dropping* it.

| Type | Frees the value? | What it allows |
| --- | --- | --- |
| `&T` | no: it borrows | reading; many `&T` at the same time |
| `&mut T` | no: it borrows | reading and writing; only one at a time |
| `Box<T>` | yes, when the box is dropped | one owner of a value on the heap |
| `Rc<T>` | yes, when the last `Rc` is dropped | several owners, one thread, read-only |
| `Arc<T>` | yes, when the last `Arc` is dropped | several owners, across threads, read-only |
| `*const T`, `*mut T` | no | nothing is checked; using it needs `unsafe` (chapter 23) |

A reference (`&T` or `&mut T`) is never null and always leads to a live value; the compiler checks this. When "no value" is possible, say so in the type: `Option<&T>`.

`Rc` means *reference counted*. The allocation stores the value and a count of how many `Rc` pointers lead to it. `Rc::clone` makes another pointer and adds one to the count. It does not copy the value. `Arc` works the same way, but its count is updated atomically, so the pointers can be given to other threads.

```text
stack                             heap
-----                             ----
hits:  u32 = 9
view:  &u32 ────────► hits        (a borrow: points at an existing value)

boxed: Box<[u8; 4]> ────────────► [1, 2, 3, 4]      freed when `boxed` is dropped

a: Rc<Vec<i32>> ──────┐
                      ├─────────► count = 2 | the vector
b: Rc<Vec<i32>> ──────┘           freed when the count reaches 0
```

### Worked example

```rust
use std::rc::Rc;
use std::sync::Arc;

fn reset(counter: &mut u32) {
    *counter = 0; // `*` reaches the value behind the reference
}

fn main() {
    let mut hits = 9;
    reset(&mut hits);
    assert_eq!(hits, 0);

    let boxed: Box<[u8; 4]> = Box::new([1, 2, 3, 4]);
    assert_eq!(boxed[2], 3); // use a box like the value inside it

    let a = Rc::new(vec![10, 20, 30]);
    let b = Rc::clone(&a); // a second pointer to the same vector
    assert!(Rc::ptr_eq(&a, &b));
    assert_eq!(Rc::strong_count(&a), 2);
    drop(b);
    assert_eq!(Rc::strong_count(&a), 1);

    let shared = Arc::new(String::from("config"));
    let for_worker = Arc::clone(&shared);
    // `move || ...` is a closure: a small function written in place (chapter 14).
    // Here it is the code that the new thread runs.
    let worker = std::thread::spawn(move || for_worker.len());
    assert_eq!(worker.join().unwrap(), 6);
    assert_eq!(shared.len(), 6); // still usable here

    let maybe: Option<&u32> = Some(&hits);
    assert_eq!(maybe, Some(&0));
}
```

### Mistakes people make

**Writing through a shared reference.**

```rust,compile_fail,E0594
fn reset(counter: &u32) {
    *counter = 0;
}

fn main() {
    let mut hits = 9;
    reset(&hits);
    println!("{hits}");
}
```

```text
error[E0594]: cannot assign to `*counter`, which is behind a `&` reference
 --> src/main.rs:2:5
  |
2 |     *counter = 0;
  |     ^^^^^^^^^^^^ `counter` is a `&` reference, so it cannot be written to
  |
help: consider changing this to be a mutable reference
  |
1 | fn reset(counter: &mut u32) {
  |                    +++
```

**Sending an `Rc` to another thread.** Its count is not safe to update from two threads, so the compiler refuses.

```rust,compile_fail,E0277
use std::rc::Rc;

fn main() {
    let shared = Rc::new(5);
    let worker = std::thread::spawn(move || println!("{shared}"));
    worker.join().unwrap();
}
```

```text
error[E0277]: `Rc<i32>` cannot be sent between threads safely
```

Fix: use `Arc`.

**Reading through a raw pointer in safe code.**

```rust,compile_fail,E0133
fn main() {
    let n = 5;
    let p: *const i32 = &n;
    let m = *p;
    println!("{m}");
}
```

```text
error[E0133]: dereference of raw pointer is unsafe and requires unsafe block
 --> src/main.rs:4:13
  |
4 |     let m = *p;
  |             ^^ dereference of raw pointer
  |
  = note: raw pointers may be null, dangling or unaligned; they can violate aliasing rules and cause data races: all of these are undefined behavior
```

Fix for now: use a reference. No exercise in this chapter needs `unsafe`.

### When to use which

Start with plain values and references. Reach for `Box` when a value must live on the heap, for example because it is large. Reach for `Rc` when several parts of a single-threaded program must keep the same value alive and none of them is the obvious owner. Use `Arc` for the same situation across threads. Leave raw pointers for unsafe code and for calling other languages.

### Read more

- [`Box`](https://doc.rust-lang.org/std/boxed/index.html), [`Rc`](https://doc.rust-lang.org/std/rc/index.html) and [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html) in the standard library
- [References](https://doc.rust-lang.org/std/primitive.reference.html) and [raw pointers](https://doc.rust-lang.org/std/primitive.pointer.html) in the standard library
- [Pointer types](https://doc.rust-lang.org/reference/types/pointer.html) in the Rust Reference

## Lesson 07 · Fixed-size, inline sequences

**Assumed:** lessons 01 to 06.

An array `[T; N]` is N values of type T, stored side by side. The length N is part of the type, and the compiler must know it. `[u8; 4]` and `[u8; 5]` are different types, in the same way that `u8` and `u16` are.

An array is stored *inline*: its elements are placed directly where the array itself is, with no pointer and no separate allocation. An array inside a tuple, inside another array, or inside a `Box` is simply a run of elements inside that thing. Its size is N times the size of T.

Syntax you need:

- `[31, 28, 31]` lists the elements.
- `[0; 12]` repeats one value: twelve zeros. The count must be a constant.
- `a[i]` reads or writes element `i`. The index is a `usize`, counting from 0. An index past the end panics.
- `a.get(i)` returns `Some(&element)` or `None`. It never panics.
- `a.len()` is the length.

Arrays nest. Read a nested type from the inside out: `[[i16; 5]; 2]` is 2 arrays, each holding 5 `i16` values. All ten values sit in one block:

```text
let scores: [[i16; 5]; 2]

| s00 | s01 | s02 | s03 | s04 | s10 | s11 | s12 | s13 | s14 |
  scores[0]                     scores[1]
```

### Worked example

```rust
fn main() {
    let days_in_month: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    assert_eq!(days_in_month.len(), 12);
    assert_eq!(days_in_month[1], 28);

    let mut total = 0;
    for days in days_in_month {
        total += days;
    }
    assert_eq!(total, 365);

    assert_eq!(days_in_month.get(11), Some(&31));
    assert_eq!(days_in_month.get(12), None); // no panic

    let mut board = [['.'; 3]; 3]; // three rows of three chars
    board[1][2] = 'x'; // row 1, column 2
    assert_eq!(board[1], ['.', '.', 'x']);
    assert_eq!(board[0], ['.', '.', '.']); // each row is its own value

    let scores: [[i16; 5]; 2] = [[0; 5]; 2];
    assert_eq!(scores.len(), 2);
    assert_eq!(scores[0].len(), 5);
    assert_eq!(size_of::<[u32; 12]>(), 48); // 12 elements of 4 bytes
}
```

### Mistakes people make

**The wrong length.** The length is part of the type, so this is a type error.

```rust,compile_fail,E0308
fn main() {
    let week: [u8; 7] = [0; 8];
    println!("{}", week.len());
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:2:25
  |
2 |     let week: [u8; 7] = [0; 8];
  |               -------   ^^^^^^ expected an array with a size of 7, found one with a size of 8
  |               |
  |               expected due to this
```

**An index past the end.** When the index is only known at run time, the check happens at run time:

```rust,should_panic
fn pick(days: [u32; 3], month: usize) -> u32 {
    days[month]
}

fn main() {
    println!("{}", pick([31, 28, 31], 5));
}
```

```text
thread 'main' (4521) panicked at src/main.rs:2:5:
index out of bounds: the len is 3 but the index is 5
```

When the compiler can see both the array and a constant index, it rejects the program instead:

```rust,compile_fail
fn main() {
    let days = [31, 28, 31];
    println!("{}", days[5]);
}
```

```text
error: this operation will panic at runtime
 --> src/main.rs:3:20
  |
3 |     println!("{}", days[5]);
  |                    ^^^^^^^ index out of bounds: the length is 3 but the index is 5
  |
  = note: `#[deny(unconditional_panic)]` on by default
```

Either way, no out-of-bounds read ever happens. If a bad index is a normal case in your program and not a bug, use `get` and handle `None`.

**A length from a variable.** `[0_u8; n]` with a run-time `n` does not compile. That is a job for a vector (lesson 08).

### When to use which

Use an array when the length is fixed and you know it while writing the program: twelve months, three colour channels, a four-byte header. Use a `Vec` when the length is decided at run time or changes.

A local array is usually placed on the stack, but the language does not promise where a value lives; the optimizer may keep it in registers or remove it. What is guaranteed is its size and layout.

### Read more

- [Arrays](https://doc.rust-lang.org/std/primitive.array.html) in the standard library
- [Array types](https://doc.rust-lang.org/reference/types/array.html) in the Rust Reference
- [Array and index expressions](https://doc.rust-lang.org/reference/expressions/array-expr.html) in the Rust Reference

## Lesson 08 · Growable owned sequences

**Assumed:** lessons 01 to 07.

A `Vec<T>` (a *vector*) is a sequence whose length can change while the program runs. Its elements live in one block on the heap. The `Vec` value itself is small. It has three parts:

- a *pointer* to the heap block;
- the *capacity*: how many elements the block has room for;
- the *length*: how many elements are in use.

```text
let mut v = Vec::with_capacity(4);
v.push(7);
v.push(9);

v                           heap block (room for 4)
+----------+
| pointer  | ─────────────► | 7 | 9 |   |   |
| capacity | = 4              in use  spare
| length   | = 2
+----------+
```

Only the first `length` slots hold elements. The spare slots are reserved memory with nothing in them, and you cannot read them.

`push` adds at the end. If a spare slot exists, it is used. If the block is full, the vector asks for a larger block, moves the elements over and frees the old block. How much larger is not promised, so do not write code or tests that depend on an exact capacity after growth.

The operations of this lesson:

- `vec![1, 2, 3]` and `vec![0; n]` build a vector. Here `n` may be a run-time value.
- `Vec::new()` makes an empty vector. `Vec::with_capacity(n)` makes an empty vector with room for at least `n` elements.
- `push(x)` adds at the end. `pop()` removes the last element and returns it as an `Option`.
- `insert(i, x)` and `remove(i)` work at any index. `insert` moves the element at `i` and every later one a place to the right; `remove` moves every element after `i` a place to the left.
- A range can be gathered into a vector with `collect()`. `collect` can build many kinds of collection, so the target type must be given.

### Worked example

```rust
fn main() {
    let mut queue = vec!["ada", "grace"];
    queue.push("edsger"); // at the end
    queue.insert(0, "alan"); // at the front: the others move right
    assert_eq!(queue, ["alan", "ada", "grace", "edsger"]);

    let served = queue.remove(1); // the later elements move left
    assert_eq!(served, "ada");
    assert_eq!(queue.pop(), Some("edsger"));
    assert_eq!(queue, ["alan", "grace"]);

    let countdown: Vec<u32> = (1..=5).rev().collect();
    assert_eq!(countdown, [5, 4, 3, 2, 1]);

    let mut samples: Vec<f32> = Vec::with_capacity(1_000);
    assert!(samples.capacity() >= 1_000); // room is reserved...
    assert!(samples.is_empty()); // ...but there are no elements yet
    assert_eq!(samples.first(), None);
    samples.push(0.5);
    assert_eq!(samples.len(), 1);

    let mut empty: Vec<i32> = Vec::new();
    assert_eq!(empty.pop(), None);
}
```

### Mistakes people make

**Reading a slot that was only reserved.** Capacity is not length.

```rust,should_panic
fn main() {
    let slots: Vec<u8> = Vec::with_capacity(4);
    println!("{}", slots[0]);
}
```

```text
thread 'main' (4521) panicked at src/main.rs:3:25:
index out of bounds: the len is 0 but the index is 0
```

If you want four zero elements, write `vec![0_u8; 4]`.

**Forgetting `mut`.** Changing a vector needs a mutable binding.

```rust,compile_fail,E0596
fn main() {
    let names = Vec::new();
    names.push("ada");
    println!("{}", names.len());
}
```

```text
error[E0596]: cannot borrow `names` as mutable, as it is not declared as mutable
 --> src/main.rs:3:5
  |
3 |     names.push("ada");
  |     ^^^^^ cannot borrow as mutable
  |
help: consider changing this to be mutable
  |
2 |     let mut names = Vec::new();
  |         +++
```

**`collect` without a target type.**

```rust,compile_fail,E0282
fn main() {
    let evens = (0..10).step_by(2).collect();
    println!("{}", evens.len());
}
```

```text
error[E0282]: type annotations needed
 --> src/main.rs:2:9
  |
2 |     let evens = (0..10).step_by(2).collect();
  |         ^^^^^
3 |     println!("{}", evens.len());
  |                    ----- type must be known at this point
  |
help: consider giving `evens` an explicit type
  |
2 |     let evens: Vec<_> = (0..10).step_by(2).collect();
  |              ++++++++
```

`Vec<_>` is enough: you choose the collection, and the compiler fills in the element type. A function's return type works as the clue as well (lesson 01).

### When to use which

- `vec![...]` when you can list the elements.
- `Vec::with_capacity(n)` then `push` when you know the final count in advance: the vector never has to grow.
- `Vec::new()` then `push` when you do not know the count.
- `collect()` when the elements come from a range or another sequence.

`insert(0, x)` and `remove(0)` move every element. If you need a queue that adds and removes at both ends often, chapter 16 introduces `VecDeque`.

### Read more

- [`Vec`](https://doc.rust-lang.org/std/vec/struct.Vec.html) in the standard library
- [Capacity and reallocation](https://doc.rust-lang.org/std/vec/struct.Vec.html#capacity-and-reallocation) and [Guarantees](https://doc.rust-lang.org/std/vec/struct.Vec.html#guarantees) in the `Vec` documentation
- [The `vec!` macro](https://doc.rust-lang.org/std/macro.vec.html)

## Lesson 09 · Borrow a region instead of owning a collection

**Assumed:** lessons 06 to 08.

A *slice*, written `[T]`, is a run of T values stored side by side, of some length that is not part of the type. You cannot hold a `[T]` directly, because its size is not known at compile time. You use it through a reference: `&[T]` to read, `&mut [T]` to change elements.

A slice reference is a *fat pointer*. It has two parts: the address of the first element, and the number of elements. It borrows: the elements belong to an array or a vector somewhere else, and making a slice copies nothing.

```text
let v = vec![3, 1, 4, 1, 5, 9];
let tail = &v[2..];

v:     pointer ───────► | 3 | 1 | 4 | 1 | 5 | 9 |
       capacity 6               ▲
       length   6               │
tail:  pointer ─────────────────┘
       length   4               (4, 1, 5, 9: the same memory)
```

You make a slice with a range inside the brackets. The start is included and the end is not:

| Range | Elements |
| --- | --- |
| `&v[1..4]` | indexes 1, 2, 3 |
| `&v[..2]` | from the start to index 1 |
| `&v[2..]` | from index 2 to the end |
| `&v[..]` | everything |

When a function's parameter is `&[T]`, you can pass `&array`, `&vector`, or part of either. The reference is converted to a slice reference for you. This is why most functions that only read a sequence take a slice.

Many methods you have used on arrays and vectors are really slice methods: `len`, `is_empty`, `first`, `last`, `get`, `contains`, `sort`, `reverse`, `fill`, `swap`. Both arrays and vectors lend themselves out as slices to provide them.

A `&mut [T]` can change the elements, but it cannot change how many there are. It has no capacity to grow into.

When you need elements of your own, for example to sort them without touching the caller's data, `slice.to_vec()` copies the elements into a new `Vec`. That is the one operation in this section that allocates.

### Worked example

```rust
/// The largest value, or None for an empty slice.
fn largest(values: &[i32]) -> Option<i32> {
    if values.is_empty() {
        return None;
    }
    let mut best = values[0];
    for value in &values[1..] {
        if *value > best {
            best = *value;
        }
    }
    Some(best)
}

/// Set the first `count` elements to zero, in place.
fn clear_prefix(values: &mut [i32], count: usize) {
    values[..count].fill(0);
}

fn main() {
    let array = [3, 9, 2];
    let vector = vec![4, 8, 15, 16, 23, 42];
    assert_eq!(largest(&array), Some(9)); // a whole array
    assert_eq!(largest(&vector), Some(42)); // a whole vector
    assert_eq!(largest(&vector[..3]), Some(15)); // part of a vector
    assert_eq!(largest(&[]), None);

    let tail = &vector[4..];
    assert_eq!(tail, [23, 42]);
    assert_eq!(tail.as_ptr(), &vector[4] as *const i32); // same memory

    let mut levels = [7, 7, 7, 7];
    clear_prefix(&mut levels, 2);
    assert_eq!(levels, [0, 0, 7, 7]);

    let mut copy: Vec<i32> = vector[..3].to_vec(); // an owned copy of a region
    copy.reverse();
    assert_eq!(copy, [15, 8, 4]);
    assert_eq!(vector[..3], [4, 8, 15]); // the original is unchanged
}
```

### Mistakes people make

**A range that goes past the end.** Ranges are checked like indexes.

```rust,should_panic
fn main() {
    let v = vec![1, 2, 3];
    let end = v.len() + 2;
    println!("{:?}", &v[1..end]);
}
```

```text
thread 'main' (4521) panicked at src/main.rs:4:24:
range end index 5 out of range for slice of length 3
```

**Subtracting from the length of an empty slice.** `len()` is a `usize`, which cannot go below zero.

```rust,should_panic
fn last_index(values: &[i32]) -> usize {
    values.len() - 1
}

fn main() {
    println!("{}", last_index(&[]));
}
```

```text
thread 'main' (4521) panicked at src/main.rs:2:5:
attempt to subtract with overflow
```

Check the length first, or use a method that already handles the empty case, such as `last()`.

**Asking for `&Vec<T>` when a slice would do.** The function then rejects arrays and sub-ranges for no benefit.

```rust,compile_fail,E0308
fn total(values: &Vec<i32>) -> i32 {
    let mut sum = 0;
    for value in values {
        sum += value;
    }
    sum
}

fn main() {
    let week = [3, 4, 5];
    println!("{}", total(&week));
}
```

```text
error[E0308]: mismatched types
  --> src/main.rs:11:26
   |
11 |     println!("{}", total(&week));
   |                    ----- ^^^^^ expected `&Vec<i32>`, found `&[{integer}; 3]`
   |                    |
   |                    arguments to this function are incorrect
```

Fix: declare `fn total(values: &[i32])`. Every existing caller that passes `&vector` still compiles.

### When to use which

| The function needs to... | Parameter |
| --- | --- |
| read the elements | `&[T]` |
| change elements in place | `&mut [T]` |
| add or remove elements | `&mut Vec<T>` |
| keep the collection | `Vec<T>` |

### Read more

- [Slices](https://doc.rust-lang.org/std/primitive.slice.html) in the standard library (the full list of methods)
- [Slice types](https://doc.rust-lang.org/reference/types/slice.html) in the Rust Reference
- [Range expressions](https://doc.rust-lang.org/reference/expressions/range-expr.html) in the Rust Reference

## Lesson 10 · UTF-8 bytes, scalar values, and string slices

**Assumed:** lessons 02 and 09.

Text has three levels, and "how long is this string?" has a different answer at each one.

1. **Bytes.** Rust strings are always valid UTF-8. UTF-8 stores each scalar value in 1 to 4 bytes. `len()` counts bytes, and string indexes are byte offsets.
2. **Unicode scalar values.** A `char` is one scalar value: any code point from U+0000 to U+10FFFF except the range U+D800 to U+DFFF, which is reserved for the UTF-16 encoding. A `char` always takes 4 bytes in memory, whatever its UTF-8 length. `chars()` walks a string scalar by scalar.
3. **User-perceived characters.** What a reader calls one character can be several scalars: a letter followed by a combining accent, or an emoji followed by a skin-tone modifier. The standard library does not split text at this level; external crates do.

| Scalar range | UTF-8 bytes | Example |
| --- | --- | --- |
| U+0000 to U+007F | 1 | `a` |
| U+0080 to U+07FF | 2 | `ë` |
| U+0800 to U+FFFF | 3 | `€` |
| U+10000 to U+10FFFF | 4 | `🙂` |

```text
text          Z      o      ë         !
scalar        U+5A   U+6F   U+EB      U+21
bytes         5A     6F     C3 AB     21
byte offset   0      1      2  3      4        len() = 5

char boundaries: 0, 1, 2, 4, 5      offset 3 is inside ë
```

A string slice `&text[a..b]` uses byte offsets. Both ends must be *char boundaries*: offsets where a scalar starts, or the end of the string. Cutting inside a scalar would produce invalid UTF-8, so Rust panics instead.

Tools that never panic:

- `text.is_char_boundary(i)` answers whether offset `i` is safe to cut at.
- `text.get(a..b)` returns `Some(&str)` or `None`.
- `text.char_indices()` yields each scalar together with the byte offset where it starts.

### Worked example

```rust
/// The first `count` scalars of `text`, as a borrowed slice.
fn prefix(text: &str, count: usize) -> &str {
    // `nth(count)` skips `count` items and returns the next one, if any.
    match text.char_indices().nth(count) {
        Some((byte_offset, _)) => &text[..byte_offset],
        None => text, // fewer than `count` scalars: all of it
    }
}

fn main() {
    let name = "Zoë!";
    assert_eq!(name.len(), 5); // bytes
    assert_eq!(name.chars().count(), 4); // scalars
    assert_eq!(prefix(name, 3), "Zoë");
    assert_eq!(prefix(name, 10), "Zoë!");

    assert!(name.is_char_boundary(2));
    assert!(!name.is_char_boundary(3));
    assert_eq!(name.get(2..4), Some("ë"));
    assert_eq!(name.get(2..3), None); // would cut ë in half

    assert_eq!('ë'.len_utf8(), 2);
    assert_eq!('ë' as u32, 0xEB);
    assert_eq!(char::from_u32(0x20AC), Some('€'));
    assert_eq!(char::from_u32(0x11_0000), None); // above the last code point

    // One character to a reader; two scalars; eight bytes.
    let thumbs_up = "👍🏽";
    assert_eq!(thumbs_up.chars().count(), 2);
    assert_eq!(thumbs_up.len(), 8);
}
```

### Mistakes people make

**Indexing a string with a number.** A byte offset is not a character, so `str` has no integer index.

```rust,compile_fail,E0277
fn main() {
    let name = "Zoë";
    let first = name[0];
    println!("{first}");
}
```

```text
error[E0277]: the type `str` cannot be indexed by `{integer}`
   --> src/main.rs:3:22
    |
  3 |     let first = name[0];
    |                      ^ string indices are ranges of `usize`
    |
    = help: the trait `SliceIndex<str>` is not implemented for `{integer}`
    = note: you can use `.chars().nth()` or `.bytes().nth()`
```

Say which level you mean: `name.chars().next()` for the first scalar, `name.as_bytes()[0]` for the first byte.

**Slicing inside a scalar.**

```rust,should_panic
fn main() {
    let name = String::from("Zoë");
    let end = name.len() - 1;
    println!("{}", &name[..end]);
}
```

```text
thread 'main' (4521) panicked at src/main.rs:4:25:
end byte index 3 is not a char boundary; it is inside 'ë' (bytes 2..4 of string)
```

"Remove the last byte" is not "remove the last character". Lesson 11 shows the method that removes a whole scalar.

**Using `len()` as the number of characters.** `"Zoë!".len()` is 5. Code that pads, truncates or validates by `len()` works with ASCII test data and then breaks with real names.

### When to use which

- Bytes (`len`, `as_bytes`) for storage sizes, buffer limits and protocols.
- Scalars (`chars`, `char_indices`) for most text processing: searching, classifying, converting.
- User-perceived characters (an external crate) for cursor movement, visible width, and cutting text for display.

### Read more

- [`char`](https://doc.rust-lang.org/std/primitive.char.html) in the standard library, including [`char::from_u32`](https://doc.rust-lang.org/std/primitive.char.html#method.from_u32)
- [`str`](https://doc.rust-lang.org/std/primitive.str.html) in the standard library, including [`is_char_boundary`](https://doc.rust-lang.org/std/primitive.str.html#method.is_char_boundary) and [`char_indices`](https://doc.rust-lang.org/std/primitive.str.html#method.char_indices)
- [Textual types](https://doc.rust-lang.org/reference/types/textual.html) in the Rust Reference

## Lesson 11 · Building text and writing literals

**Assumed:** lessons 08 to 10.

Rust has two main text types, and they relate exactly like `Vec<T>` and `&[T]`.

- `String` owns its text. It is a pointer, a capacity and a length, with a heap block of UTF-8 bytes. It can grow.
- `&str` borrows text. It is a pointer and a length. The bytes belong to something else: a `String`, or the program file itself.

A string literal such as `"hi"` is a `&'static str`. Its bytes are stored inside the compiled program, and `'static` says that they stay valid for the whole run.

```text
let owned = String::from("hello world");
let word: &str = &owned[6..];
let literal: &str = "hi";

owned:    pointer ──────► heap:  h e l l o _ w o r l d
          capacity 11                        ▲
          length   11                        │
word:     pointer ───────────────────────────┘
          length   5
literal:  pointer ──────► read-only program data:  h i
          length   2
```

**Building text.** `String::from("x")` and `"x".to_string()` copy a `&str` into a new `String`. `format!` builds a `String` with the same syntax as `println!`. On an existing `String`, `push(char)` and `push_str(&str)` append in place. `["a", "b"].join("/")` and `.concat()` build one `String` from a slice of pieces.

**Taking text apart.** `trim()`, `split(',')` and `lines()` return `&str` pieces that point into the original. They allocate nothing. `pop()` removes the last scalar from a `String` and returns it as an `Option<char>`.

**Literal forms.**

| Form | Meaning |
| --- | --- |
| `"a\tb\n"` | escapes: `\t` tab, `\n` newline, `\\` backslash, `\"` quote, `\u{e9}` a scalar by number |
| `r"C:\temp"` | raw: backslashes are ordinary characters |
| `r#"say "hi""#` | raw with `#`: may contain quotes |

A literal may continue over several source lines. A backslash at the end of a line removes the line break and the leading spaces of the next line.

**Two ways to print.** `{}` prints the text itself. `{:?}` prints it as a literal, with quotes and escapes; use it in logs and test messages, where invisible characters matter.

### Worked example

```rust
/// "temp" and 21.46 become "temp=21.5°C".
fn reading_label(name: &str, celsius: f64) -> String {
    format!("{name}={celsius:.1}°C")
}

/// The title, a newline, and one '=' for each scalar of the title.
fn underline(title: &str) -> String {
    let mut out = String::from(title);
    out.push('\n');
    for _ in title.chars() {
        out.push('=');
    }
    out
}

fn main() {
    assert_eq!(reading_label("temp", 21.46), "temp=21.5°C");
    assert_eq!(underline("Résumé"), "Résumé\n======");

    let path = ["usr", "local", "bin"].join("/");
    assert_eq!(path, "usr/local/bin");

    let raw = "  42 apples \n";
    let trimmed: &str = raw.trim(); // a view into `raw`, not a new String
    assert_eq!(trimmed, "42 apples");

    let mut word = String::from("naï");
    assert_eq!(word.pop(), Some('ï')); // the whole two-byte scalar
    assert_eq!(word, "na");

    assert_eq!("C:\\temp", r"C:\temp");
    assert_eq!("say \"hi\"", r#"say "hi""#);
    assert_eq!(format!("{:?}", "a\nb"), r#""a\nb""#);
}
```

### Mistakes people make

**Returning a literal where a `String` is expected.**

```rust,compile_fail,E0308
fn greeting() -> String {
    "hello"
}

fn main() {
    println!("{}", greeting());
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:2:5
  |
1 | fn greeting() -> String {
  |                  ------ expected `String` because of return type
2 |     "hello"
  |     ^^^^^^^ expected `String`, found `&str`
  |
help: try using a conversion method
  |
2 |     "hello".to_string()
  |            ++++++++++++
```

**Adding two `&str` values.** `+` appends to an owned `String` on the left.

```rust,compile_fail,E0369
fn main() {
    let stem = "data";
    let extension = ".csv";
    let file = stem + extension;
    println!("{file}");
}
```

```text
error[E0369]: cannot add `&str` to `&str`
 --> src/main.rs:4:21
  |
4 |     let file = stem + extension;
  |                ---- ^ --------- &str
  |                |    |
  |                |    `+` cannot be used to concatenate two `&str` strings
  |                &str
  |
  = note: string concatenation requires an owned `String` on the left
```

`format!("{stem}{extension}")` is usually clearer than `+`.

**Thinking `mut` makes borrowed text growable.** `let mut s = "hi";` lets `s` point at a *different* piece of text later. The text itself is still borrowed and read-only.

```rust,compile_fail,E0599
fn main() {
    let mut s = "hi";
    s.push('!');
    println!("{s}");
}
```

```text
error[E0599]: no method named `push` found for reference `&str` in the current scope
 --> src/main.rs:3:7
  |
3 |     s.push('!');
  |       ^^^^ method not found in `&str`
```

Fix: `let mut s = String::from("hi");`.

### When to use which

- Parameter: `&str`. Callers can pass a literal, a `&String`, or a piece of either.
- Return `String` when the function builds new text.
- Return `&str` when the result is a piece of the input, as `trim` does.
- In a loop, `push_str` into one `String` instead of creating a new `String` on every pass.

### Read more

- [`String`](https://doc.rust-lang.org/std/string/struct.String.html) and [`str`](https://doc.rust-lang.org/std/primitive.str.html) in the standard library
- [`std::fmt`](https://doc.rust-lang.org/std/fmt/index.html) for the formatting syntax
- [String literals](https://doc.rust-lang.org/reference/tokens.html#string-literals) and [raw string literals](https://doc.rust-lang.org/reference/tokens.html#raw-string-literals) in the Rust Reference

## Lesson 12 · Pick the representation at a system boundary

**Assumed:** lessons 09 to 11.

A `String` makes a promise: its bytes are valid UTF-8. The world outside your program makes no such promise.

- A file can contain any bytes.
- A file name on Linux can be almost any byte sequence. On Windows it is a sequence of 16-bit units that need not be valid text.
- A C library expects text as bytes that end with a zero byte.

The *boundary* is the place where such data enters or leaves your program. Rust gives each kind of data its own pair of types, one that owns and one that borrows:

| Data | Owns | Borrows | Literal |
| --- | --- | --- | --- |
| text (UTF-8) | `String` | `&str` | `"abc"` |
| any bytes | `Vec<u8>` | `&[u8]` | `b"abc"`, and `b'a'` for one byte |
| a file-system path | `PathBuf` | `&Path` | none: `Path::new("a/b")` |
| a string from the operating system | `OsString` | `&OsStr` | none: `OsStr::new("a")` |
| a string for C code | `CString` | `&CStr` | `c"abc"` |

Each pair works like `String` and `&str`. Going from text to bytes, to a path or to an OS string always works, because valid UTF-8 is acceptable there. Going back to text is a checked step: `path.to_str()` returns an `Option`, `String::from_utf8(bytes)` returns a `Result`, and `to_string_lossy()` replaces anything invalid with the replacement character U+FFFD.

A C string has one more rule. It ends at the first zero byte, so it cannot contain a zero byte in the middle. `CString::new` checks this, so even text-to-C-string is a checked step. A `c"..."` literal adds the final zero for you.

Inside a byte literal, `\xHH` writes one byte by its hexadecimal value. This is how you write a byte that is not ASCII: `b"\xff\xd8"` is two bytes.

Each pair has a method in each direction. The borrowed-to-owned direction copies the data. The owned-to-borrowed direction copies nothing.

| Pair | Borrowed to owned | Owned to borrowed |
| --- | --- | --- |
| `&str` / `String` | `to_string()` | `as_str()` |
| `&[u8]` / `Vec<u8>` | `to_vec()` | `as_slice()` |
| `&Path` / `PathBuf` | `to_path_buf()` | `as_path()` |
| `&OsStr` / `OsString` | `to_os_string()` | `as_os_str()` |
| `&CStr` / `CString` | `to_owned()` | `as_c_str()` |

`to_owned()` works on the borrowed type of every pair. A plain `&` on the owned value also gives the borrowed type wherever the compiler can see that one is expected: `&string` can be passed as a `&str`, and `&path_buf` as a `&Path`.

### Type aliases

`type Name = ExistingType;` gives an existing type a second name. It does not create a new type. Both names can be used in each other's place, and the alias adds no checks. An alias makes a long type shorter to write. It does not protect you from mixing values up.

### Worked example

```rust
use std::ffi::{CString, OsStr};
use std::path::{Path, PathBuf};

type Millis = u64;
type Meters = u64;

fn log_file(dir: &Path, day: u32) -> PathBuf {
    dir.join(format!("day-{day:02}.log"))
}

fn main() {
    let file = log_file(Path::new("logs"), 7);
    assert_eq!(file.file_name(), Some(OsStr::new("day-07.log")));
    assert_eq!(file.extension(), Some(OsStr::new("log")));
    assert!(file.to_str().is_some()); // this path happens to be valid UTF-8

    let owned_dir: PathBuf = Path::new("logs").to_path_buf(); // borrowed to owned
    assert_eq!(owned_dir.as_path(), Path::new("logs")); // and back, without a copy
    assert_eq!(log_file(&owned_dir, 7), file); // `&PathBuf` is accepted as `&Path`

    let header: &[u8; 6] = b"GIF89a"; // a byte string literal is a byte array
    assert_eq!(header[0], b'G');
    let jpeg_start: Vec<u8> = b"\xff\xd8\xff".to_vec(); // three bytes, written in hex
    assert_eq!(jpeg_start, [0xff, 0xd8, 0xff]);
    assert!(String::from_utf8(jpeg_start).is_err()); // bytes, but not text

    let name = CString::new("libdemo").unwrap();
    assert_eq!(name.as_bytes_with_nul().len(), 8); // 7 letters and the final zero
    assert!(CString::new("a\0b").is_err()); // a zero in the middle

    let timeout: Millis = 250;
    let distance: Meters = timeout; // compiles: both names mean u64
    assert_eq!(distance, 250);
}
```

The last two lines are a bug that the compiler cannot see. If milliseconds and meters must not mix, you need two real types. Chapter 9 shows how to make them.

### Mistakes people make

**Printing a path with `{}`.** A path may not be valid text, so it has no plain display form.

```rust,compile_fail,E0277
use std::path::Path;

fn main() {
    let notes = Path::new("notes.txt");
    println!("{}", notes);
}
```

```text
error[E0277]: `Path` doesn't implement `std::fmt::Display`
 --> src/main.rs:5:20
  |
5 |     println!("{}", notes);
  |               --   ^^^^^ `Path` cannot be formatted with the default formatter; call `.display()` on it
  |               |
  |               required by this formatting parameter
  |
  = help: the trait `std::fmt::Display` is not implemented for `Path`
  = note: call `.display()` or `.to_string_lossy()` to safely print paths, as they may contain non-Unicode data
```

Fix: `println!("{}", notes.display());`.

**A non-ASCII character in a byte literal.** A byte literal is bytes, not text, so each character must be one ASCII byte or an escape.

```rust,compile_fail
fn main() {
    let bytes = b"naïve";
    println!("{}", bytes.len());
}
```

```text
error: non-ASCII character in byte string literal
 --> src/main.rs:2:21
  |
2 |     let bytes = b"naïve";
  |                     ^ must be ASCII
```

If you want the UTF-8 bytes of some text, write the text and ask for its bytes: `"naïve".as_bytes()`.

**Keeping paths in a `String`.** Building a path with `format!("{dir}/{name}")` assumes `/` as the separator and assumes that every file name is valid UTF-8. `Path::join` makes neither assumption.

### When to use which

Convert at the boundary and nowhere else. Inside the program, keep the type that states what you know: `String` once you have checked that the data is text, `PathBuf` for anything you will give back to the file system, `Vec<u8>` for data you only store or forward.

### Read more

- [`std::path`](https://doc.rust-lang.org/std/path/index.html) and [`std::ffi`](https://doc.rust-lang.org/std/ffi/index.html) in the standard library
- [`OsString`](https://doc.rust-lang.org/std/ffi/struct.OsString.html) and [`CStr`](https://doc.rust-lang.org/std/ffi/struct.CStr.html)
- [Byte string literals](https://doc.rust-lang.org/reference/tokens.html#byte-string-literals), [C string literals](https://doc.rust-lang.org/reference/tokens.html#c-string-literals) and [type aliases](https://doc.rust-lang.org/reference/items/type-aliases.html) in the Rust Reference

## Lesson 14 · Checked conversions and numeric edge cases

**Assumed:** lessons 02 to 04, 09 and 11. Do this lesson before the capstone.

Lessons 02 to 04 gave you `as`. This lesson adds the tools you use when a wrong number must not pass silently.

### Three ways to convert a number

| Tool | Compiles for | If the value does not fit |
| --- | --- | --- |
| `T::from(x)` | pairs where every value fits (`u8` to `u32`, `i32` to `f64`) | cannot happen |
| `T::try_from(x)` | any two integer types | returns `Err`, and you decide what to do |
| `x as T` | any two numeric types | gives a defined value that may surprise you |

`try_from` returns a `Result`. Use `match` on it, or call `.ok()` to turn it into an `Option`.

A float-to-integer `as` cast has three rules. The fraction is dropped. A value outside the integer's range becomes the nearest limit (`MIN` or `MAX`). NaN becomes 0.

### Four edge cases

**The smallest signed value has no positive partner.** `i16` runs from −32,768 to 32,767. So `-i16::MIN`, `i16::MIN.abs()` and `i16::MIN / -1` all overflow. Use `unsigned_abs()` (returns the unsigned type), `checked_abs()`, `checked_neg()` and `checked_div()`.

**`%` is a remainder, not a modulo.** `/` rounds toward zero, and the result of `%` has the sign of the left operand, so it can be negative. With a positive divisor, `rem_euclid` always returns a value from 0 up to (not including) the divisor, and `div_euclid` is the matching division. Use these for anything that goes around: clock positions, days of the week, indexes into a ring.

**Floats have no complete order under `<`.** NaN is neither smaller nor larger than anything, so `f64` cannot be sorted with `sort()`. `a.total_cmp(&b)` gives a complete order: −∞, the negative numbers, −0.0, +0.0, the positive numbers, +∞. A NaN comes after all of these when its sign bit is clear, and before all of them when its sign bit is set. It returns an `Ordering` (`Less`, `Equal` or `Greater`). Slices have `sort_by`, which takes the comparison to use.

**Bytes have an order.** A `u32` is four bytes. Which byte comes first in memory depends on the processor, so anything written to a file or a network must name the order. *Big-endian* puts the most significant byte first. *Little-endian* puts the least significant byte first.

```text
the u32 value 0x0A0B0C0D

big-endian bytes      0A 0B 0C 0D
little-endian bytes   0D 0C 0B 0A
```

`to_be_bytes` and `to_le_bytes` turn an integer into an array of bytes. `from_be_bytes` and `from_le_bytes` go back.

### Text to number

`text.parse::<T>()` returns a `Result`. For integers it accepts an optional `+` (or `-`, for signed types) followed by decimal digits, and nothing else: no spaces, no `_`, no `0x`. Trim the text first. For floats it also accepts a decimal point and an exponent such as `2.5e-3`.

### Worked example

```rust
use std::cmp::Ordering;

/// Position in a 12-month table for an offset that may be negative.
fn month_index(offset: i32) -> usize {
    // rem_euclid(12) is always in 0..12, so this cast loses nothing.
    offset.rem_euclid(12) as usize
}

/// The largest value in the total order, or None for an empty slice.
fn largest(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut best = values[0];
    for &value in &values[1..] {
        if value.total_cmp(&best) == Ordering::Greater {
            best = value;
        }
    }
    Some(best)
}

fn main() {
    let small: u8 = 200;
    assert_eq!(u32::from(small), 200); // always fits
    assert_eq!(i8::try_from(small).ok(), None); // 200 is above i8::MAX
    assert_eq!(i8::try_from(100_u8).ok(), Some(100));
    assert_eq!(small as i8, -56); // no error: same bits, new meaning

    assert_eq!(1e10_f64 as i32, i32::MAX); // too large: the nearest limit
    assert_eq!(-3.99_f64 as i32, -3); // the fraction is dropped
    assert_eq!(-1.0_f64 as u32, 0); // below the range: the nearest limit

    assert_eq!(i16::MIN.unsigned_abs(), 32_768);
    assert_eq!(i16::MIN.checked_abs(), None);
    assert_eq!(i16::MIN.checked_div(-1), None);
    assert_eq!(7_i16.checked_div(0), None);

    assert_eq!((-13 / 5, -13 % 5), (-2, -3));
    assert_eq!(((-13_i32).div_euclid(5), (-13_i32).rem_euclid(5)), (-3, 2));
    assert_eq!(month_index(-1), 11);
    assert_eq!(month_index(14), 2);

    assert_eq!(largest(&[2.5, -1.0, 9.0]), Some(9.0));
    let mut times: Vec<f64> = vec![2.5, 0.5, 9.0];
    times.sort_by(|a, b| b.total_cmp(a)); // a closure: here, largest first
    assert_eq!(times, [9.0, 2.5, 0.5]);

    assert_eq!(0x0A0B_0C0D_u32.to_be_bytes(), [0x0A, 0x0B, 0x0C, 0x0D]);
    assert_eq!(0x0A0B_0C0D_u32.to_le_bytes(), [0x0D, 0x0C, 0x0B, 0x0A]);
    assert_eq!(u16::from_be_bytes([0x01, 0x00]), 256);
    assert_eq!(u16::from_le_bytes([0x01, 0x00]), 1);

    assert_eq!("-42".parse::<i32>(), Ok(-42));
    assert!("42 ".parse::<i32>().is_err()); // parse does not trim
    assert!("4_2".parse::<i32>().is_err());
    assert!("300".parse::<u8>().is_err()); // out of range for the target
    assert_eq!("2.5".parse::<f64>(), Ok(2.5));
    let level: u8 = "7".parse().unwrap_or(0); // the annotation picks the target
    assert_eq!(level, 7);
}
```

`sort_by` accepts a closure, as above, or the name of a function with the right shape, such as `f64::total_cmp`.

### Mistakes people make

**Using the result of `try_from` as if it were the number.**

```rust,compile_fail,E0308
fn main() {
    let level: u8 = u8::try_from(300_i32);
    println!("{level}");
}
```

```text
error[E0308]: mismatched types
 --> src/main.rs:2:21
  |
2 |     let level: u8 = u8::try_from(300_i32);
  |                --   ^^^^^^^^^^^^^^^^^^^^^ expected `u8`, found `Result<u8, TryFromIntError>`
  |                |
  |                expected due to this
```

The `Result` is the point: handle the `Err` case. (`u8::from(300_i32)` fails differently: ``the trait bound `u8: From<i32>` is not satisfied``, because that conversion could lose data.)

**Sorting floats with `sort()`.**

```rust,compile_fail,E0277
fn main() {
    let mut temps = [2.5, -1.0, 9.0];
    temps.sort();
    println!("{temps:?}");
}
```

```text
error[E0277]: the trait bound `{float}: Ord` is not satisfied
   --> src/main.rs:3:11
    |
  3 |     temps.sort();
    |           ^^^^ the trait `Ord` is not implemented for `{float}`
```

`Ord` is the name for "has a complete order". Floats do not have one under `<`, so use `sort_by` with `total_cmp`.

**Parsing without a target type.**

```rust,compile_fail,E0284
fn main() {
    let count = "5".parse().unwrap();
    println!("{count}");
}
```

```text
error[E0284]: type annotations needed
    --> src/main.rs:2:9
     |
   2 |     let count = "5".parse().unwrap();
     |         ^^^^^       ----- type must be known at this point
```

Fix: `"5".parse::<u32>()`, or annotate the binding.

### When to use which

`from` when it compiles. `try_from` when the value comes from outside and might not fit. `as` only when dropping bits, dropping a fraction or stopping at a limit is what you want, and then say so in a comment.

### Read more

- [`TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html) in the standard library
- [`rem_euclid`](https://doc.rust-lang.org/std/primitive.i32.html#method.rem_euclid), [`unsigned_abs`](https://doc.rust-lang.org/std/primitive.i32.html#method.unsigned_abs) and [`to_be_bytes`](https://doc.rust-lang.org/std/primitive.u32.html#method.to_be_bytes)
- [`f64::total_cmp`](https://doc.rust-lang.org/std/primitive.f64.html#method.total_cmp) and [`str::parse`](https://doc.rust-lang.org/std/primitive.str.html#method.parse)
- [Numeric cast semantics](https://doc.rust-lang.org/reference/expressions/operator-expr.html#numeric-cast) in the Rust Reference

## Lesson 13 · Capstone: sensor report

**Assumed:** every lesson above.

The capstone introduces nothing new. It asks you to make, in one small program, the decisions that the lessons practised one at a time. This section describes a way of working. It does not contain the solution.

**1. Read the contract as a list of decisions.** For each field of the result, write down two things before you write code: who owns the data, and which type holds it. Text that you build is owned. Text that you only measure can stay borrowed. A sorted copy must be owned, because the caller's data must not change.

**2. Write the two totals first.** Each one is a loop with a running total. They look the same and differ in one choice: what happens when the total does not fit. Lesson 03 gives you the method families. Ask also which type the running total needs, and whether each reading must be widened *before* it is added. Test both helpers alone before you use them.

**3. Assemble the report in a fixed order.** Clean the label. Build the message. Copy and sort the readings. Call your helpers. Measure the label. Keeping this order makes each step easy to check, and it stops you from measuring the wrong piece of text.

**4. Check the edges before you run the tests.** Work these out on paper:

| Input | Question |
| --- | --- |
| no readings | What is each total? What does the message say? |
| a label with a non-ASCII character | Which of the two measurements changes? |
| a label of only whitespace | What is left after cleaning? |
| many large readings | Which total reports the problem, and how? Which one hides it on purpose? |

**5. Explain it.** A finished capstone is one where you can say, for every field, why it has that type. That explanation is what you will reuse; the code is not.

Where each decision comes from:

| Decision | Lesson |
| --- | --- |
| a total that reports overflow, and one that stops at a limit | 03 |
| widening a small integer before arithmetic | 02, 03 |
| an owned, sorted copy and a borrowed input | 08, 09 |
| bytes and scalars of the same text | 10 |
| building a message; borrowing a cleaned piece of the input | 11 |
| a name for a long tuple type | 12 |

When you are stuck, open [HINTS.md](HINTS.md) one level at a time. A model answer is in [`solutions/exercises/chapter03/`](../../solutions/exercises/chapter03/); compare with it after your own version passes.

### Read more

- [`slice::sort`](https://doc.rust-lang.org/std/primitive.slice.html#method.sort) and [`slice::to_vec`](https://doc.rust-lang.org/std/primitive.slice.html#method.to_vec)
- [`str::trim`](https://doc.rust-lang.org/std/primitive.str.html#method.trim)
- [`checked_add`](https://doc.rust-lang.org/std/primitive.u16.html#method.checked_add) and [`saturating_add`](https://doc.rust-lang.org/std/primitive.u8.html#method.saturating_add)
