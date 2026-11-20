# Chapter 3 hints — Numbers, text and arrays

[Chapter guide](README.md) · [Concepts](CONCEPTS.md) · [All hints](../../HINTS.md)

Open only the lesson you need. Try the first hint before revealing the second. Each lesson has four blocks:

- **Hint 1** and **Hint 2**: open them one at a time, after a real attempt.
- **Check your predictions**: open it after you have answered `p_predictions` and seen the result. It gives the rule behind each prediction.
- **Check your explanation**: open it after you have written your own explanation in [NOTES.md](NOTES.md).

Complete model answers are in [`solutions/exercises/chapter03/`](../../solutions/exercises/chapter03/). Use them after an honest attempt, not before.

## 01 · Inference

<details><summary>Hint 1</summary>

The returned `Vec<u16>` constrains what goes into the vector. In the numeric method task, method lookup needs a concrete integer type before it can select `abs`.

</details>
<details><summary>Hint 2</summary>

Use `Vec::new()`, a `0..3` loop, and `push`. For the second task, suffix the negative literal with `_i32` and put parentheses around the entire negative value before calling the method.

</details>
<details><summary>Check your predictions (after answering them)</summary>

- **P1 is `(4, 8)`.** With no clue, an integer literal becomes `i32` (4 bytes) and a float literal becomes `f64` (8 bytes).
- **P2 is `(-17, 17)`.** A method call binds tighter than unary minus, so `-17_i32.abs()` means `-(17_i32.abs())`.
- **P3 is `7`.** The line `let narrow: u8 = a;` makes `a` a `u8`. The value 1 in eight bits is `0000_0001`, which has seven leading zeros.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Function signatures (parameter and return types) must always be written; inside a body, the compiler infers local types from how values are used. The return type `Vec<u16>` tells inference that the vector returned at the end is a `Vec<u16>`, so `Vec::new()` and the pushed values become u16, even though no local has an annotation. Without any constraint, an integer literal defaults to i32 and a float literal to f64. A method call needs a concrete receiver type at that point, so `(-17).abs()` is ambiguous until a suffix or `i32::abs` decides it. A method call binds tighter than unary minus, so `-17_i32.abs()` is `-(17_i32.abs())`.

</details>

## 02 · Integers

<details><summary>Hint 1</summary>

Use the prefixes `0x`, `0o`, and `0b`. A cast to a narrower integer keeps the low bits. A same-width unsigned-to-signed cast interprets those bits as a signed value.

</details>
<details><summary>Hint 2</summary>

Use `as u8` for the deliberately truncated low byte, `as i8` for the signed view, and the integer method `count_ones` for the number of set bits. Its return type is `u32`.

</details>
<details><summary>Check your predictions (after answering them)</summary>

- **P1 is `(44, -1)`.** 300 is `1_0010_1100` in binary; the low eight bits are `0010_1100`, which is 44. 255 is `1111_1111`; read as an `i8`, that pattern is −1.
- **P2 is `(255, -1, 4_294_967_295)`.** A same-width cast keeps the bits (`1111_1111` is 255 as a `u8`). Widening a signed value copies its sign bit into the new high bits, so it is still −1 as an `i32`, and the same 32 one bits are `u32::MAX` when read as unsigned.
- **P3 is `(8, 16, 64)`.** A `u64` takes 8 bytes, an `i128` takes 16, and `usize` is 64 bits wide on a 64-bit target.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A numeric value is what the bits mean; the bit pattern is the bits themselves. `255_u8` and `-1_i8` share the pattern `1111_1111`. `as` between integers never fails: narrowing keeps the low bits (dropping the rest), same-width signed/unsigned casts reinterpret the bits, and widening a signed value sign-extends it, so `-1_i8 as u32` is `u32::MAX`. Information is lost whenever the value does not fit the target. In task B, discarding the high bits *is* the specification ("the low byte"), so truncation is correct there; when you need the same number or an error, use `u8::try_from` instead (lesson 14).

</details>

## 03 · Overflow

<details><summary>Hint 1</summary>

Each requested behavior has a named method on integers. Ordinary `+` is profile dependent, so it cannot stand in for these explicit policies.

</details>
<details><summary>Hint 2</summary>

Look up `wrapping_add`, `checked_add`, `saturating_add`, `overflowing_add`, `strict_mul`, `checked_shl`, and `wrapping_shl`. For midpoint, widen both operands **before** adding, or use the integer `midpoint` method. For shifts on `u8`, a distance of 8 is already out of range, even when the value is zero.

</details>
<details><summary>Check your predictions (after answering them)</summary>

- **P1 is `(44, -56)`.** Wrapping keeps the low eight bits. 300 − 256 = 44. For the signed case, 200 does not fit in an `i8`; 200 − 256 = −56.
- **P2 is `(None, -128, 127)`.** +128 does not fit in an `i8`. The checked form reports that, the wrapping form lands on −128 again, and the saturating form stops at `i8::MAX`.
- **P3 is `(Some(240), 2)`.** `checked_shl` checks only the distance. Shifting 255 left by 4 loses the top four bits and gives `1111_0000` (240) with no report. `wrapping_shl(33)` on a 32-bit type uses 33 mod 32 = 1 as the distance.
- **P4 is `(255, true)`.** The result wraps around to 255, and the flag says that overflow happened.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Brightness should saturate (clamp at the brightest value), a cyclic counter should wrap, and a buffer size should use checked arithmetic and reject the request when it returns `None`. A checked shift checks the *distance*: `checked_shl` returns `None` only when the distance is at least the bit width, and bits shifted out of the top are silently lost. Plain `+` panics on overflow in debug builds and wraps in release builds, so it is never a policy. Casting after the addition is too late, because the overflow already happened in the narrow type; widen the operands first.

</details>

## 04 · Floating point

<details><summary>Hint 1</summary>

Classify a NaN with a method, not equality. IEEE floating-point division by zero produces infinities here. Preserve the sign of zero in the reciprocal exercise.

</details>
<details><summary>Hint 2</summary>

Use `is_finite`, `is_infinite`, and `is_nan`; `std::f64::consts::PI`; and explicit conversions on both inputs of the scaling expression (`f64::from` works for both). Calculate the reciprocals from `0.0_f64` and `-0.0_f64`.

</details>
<details><summary>Check your predictions (after answering them)</summary>

- **P1 is `(false, true)`.** In `f64`, the rounded sum is 0.30000000000000004, which is not the `f64` nearest to 0.3. In `f32`, the rounded sum happens to be the same value as the `f32` nearest to 0.3. Neither result is "more correct"; equality after arithmetic depends on how the rounding falls.
- **P2 is `(false, true, 1.0)`.** NaN is not equal to anything, not even to a copy of itself. The two zeros compare as equal. `max` ignores a NaN operand and returns the other one.
- **P3 is `(-2.0, -2.0, 3.0, -1.0)`.** `floor` goes down, `round` goes to the nearest whole number with halves away from zero, and `trunc` drops the fraction.
- **P4 is `(16_777_216, false)`.** An `f32` has 24 significant bits, so 2^24 + 1 cannot be stored and becomes 2^24. Widening the `f32` nearest to 0.1 gives exactly that `f32` value, which is not the `f64` nearest to 0.1.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Most decimal fractions (0.1, 0.2) have no exact binary representation, so each operation rounds and results differ from the decimal ideal in the last bits; tests compare with a tolerance instead of `==`. NaN is not equal to anything, including itself, so classify it with `is_nan`; `0.0 == -0.0` is true even though their reciprocals are opposite infinities. Converting an f32 to f64 is exact, but it cannot restore digits that were rounded away when the f32 was created, which is why task D converts *before* multiplying. f32 has a 24-bit significand, so integers above 2^24 can no longer all be represented.

</details>

## 05 · Booleans, tuples, unit

<details><summary>Hint 1</summary>

A condition must already be `bool`. A Boolean can be cast to an integer, but an integer cannot be cast to `bool`. A pair of return values can be destructured with `let (left, right) = ...`.

</details>
<details><summary>Hint 2</summary>

Use a comparison with zero, a Boolean cast, and `split_at`. For the validation function, `assert!` takes a condition and then an optional message in `format!` style: `assert!(condition, "packet size {size} ...")`. The condition can be two comparisons joined with `&&`, or `(1..=1_500).contains(&size)`, which clippy prefers. With no explicit return value, the function returns `()` when the assertion succeeds.

</details>
<details><summary>Check your predictions (after answering them)</summary>

- **P1 is `(0, 8, 2)`.** Unit has no fields, so it takes no bytes. In `(u8, u32)` the `u32` must start at a multiple of 4, so three padding bytes are added and the total is 8. Two `u8` fields need no padding.
- **P2 is `(true, false)`.** Tuples compare field by field from the left. `1 < 2` decides the first comparison. In the second, the first fields are equal and `"b" < "a"` is false.
- **P3 is `[false, true, true, true]`.** `false` is ordered before `true`.
- **P4 is `(2, 3)`.** The tuple on the right is built from the old values (`b` is 2 and `a + b` is 3) before either name is rebound.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A tuple's type lists each field's type, so mixed types are fine: the compiler knows the type at every position. `t.0` names a field at compile time; `t[i]` would need a run-time index, and the compiler could not know which type the result has. Conditions must be `bool`; Rust never treats integers as truthy. The unit type `()` has exactly one value, also written `()`; functions and blocks without a final expression produce it.

</details>

## 06 · Pointer types

<details><summary>Hint 1</summary>

Dereference a mutable reference to change its referent. Two separately allocated values with equal contents do not demonstrate shared ownership of one allocation.

</details>
<details><summary>Hint 2</summary>

Use `Box::new`, then `Rc::new` plus `Rc::clone`, and `Arc::new` plus `Arc::clone`. The clone here is of the smart pointer. For the optional borrow, return `Some(value)` or `None`. No exercise requires `unsafe`.

</details>
<details><summary>Check your predictions (after answering them)</summary>

- **P1 is `(1, true)`.** A `Box<i32>` is one address, so one machine word. A reference is never null, so `Option<&i32>` uses the null address to mean `None` and needs no extra space.
- **P2 is `(3, 2)`.** Inside the block three pointers (`a`, `b`, `c`) share the allocation. `c` is dropped at the end of the block.
- **P3 is `true`.** Moving a `Box` copies the pointer. The value on the heap stays where it is.
- **P4 is `(false, true)`.** A raw pointer made from a reference is not null; `std::ptr::null()` is.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Tuples and arrays store their fields inline in one block of memory; Rust allocates on the heap only when you ask, for example with `Box`, `Vec`, or `Rc`. `&T` and `&mut T` borrow and never free anything; `Box<T>` uniquely owns a heap value and frees it when dropped; `Rc<T>` and `Arc<T>` free it when the last clone is dropped, and Arc's count is atomic so it can cross threads. References are never null, so absence is spelled `Option<&T>`, which costs no extra space because `None` uses the null bit pattern. Raw pointers may be null or dangling, and dereferencing them requires `unsafe`.

</details>

## 07 · Arrays

<details><summary>Hint 1</summary>

An array's length belongs to its type. Repeat-value syntax can initialize arrays, including arrays whose elements are themselves small arrays.

</details>
<details><summary>Hint 2</summary>

Start with `[0; 8]` for the buffer. The supplied row contains only `i16` values, so it can be repeated with `[row; 2]`. Use `get` to return an optional reference instead of panicking on an invalid index.

</details>
<details><summary>Check your predictions (after answering them)</summary>

- **P1 is `(48, 0)`.** Three rows of four `u32` values are twelve values of 4 bytes each. An array with no elements takes no bytes.
- **P2 is `(2, 3)`.** Read the type from the inside out: `[[u8; 3]; 2]` is two arrays of three bytes.
- **P3 is `(Some(&30), None)`.** Index 2 is the last element; index 3 is one past the end, and `get` reports that with `None`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The length is part of an array's type (`[u8; 8]` is not `[u8; 9]`), so the compiler must know it to lay out the value; a run-time length needs a `Vec`. Because arrays store elements inline, a nested array like `[[i16; 3]; 2]` is one contiguous block of 12 bytes inside whatever contains it, with no heap allocation. Indexing out of range panics at run time (or is rejected at compile time for a constant index), while `get` returns `None`. Where a local array actually lives is up to the compiler; only its size and layout are guaranteed.

</details>

## 08 · Vectors

<details><summary>Hint 1</summary>

Reserving storage does not create elements. Capture the initial length and capacity before pushing anything. A vector's capacity is at least its length; no particular growth factor is promised.

</details>
<details><summary>Hint 2</summary>

Use `collect`, `insert`, `remove`, `push`, `Vec::with_capacity`, `len`, `capacity`, and `pop`. In the queue task, insert first, then remove index zero. In the capacity task, loop with `for i in 0..count` and push `i as u8` into the vector you reserved; do not build a second vector.

</details>
<details><summary>Check your predictions (after answering them)</summary>

- **P1 is `(0, true)`.** `with_capacity` reserves room and creates no elements. The capacity is at least what you asked for; it may be more.
- **P2 is `[9, 2]`.** After `insert(1, 9)`: `[1, 9, 2, 3]`. After `remove(0)`: `[9, 2, 3]`. After `pop()`: `[9, 2]`.
- **P3 is `(0, 1)`.** `Vec::new()` does not allocate; the first allocation happens when an element is pushed. `with_capacity(4)` allocates its block at once.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A `Vec` is a pointer, a capacity, and a length: capacity is how many elements fit in the current allocation, length is how many are initialized and usable. Reserving with `with_capacity` creates no elements, so indexing it still panics. When a push exceeds the capacity, the vector allocates a larger buffer and moves its elements; the growth factor is an implementation detail, so tests should not depend on it. `pop` removes the last element in O(1); `insert` and `remove` at index i shift every element after i.

</details>

## 09 · Slices

<details><summary>Hint 1</summary>

An array, a vector, and a subrange can all supply `&[i32]`. A subslice borrows existing elements. Be careful before subtracting from the length of an empty slice.

</details>
<details><summary>Hint 2</summary>

For lengths below two, return a zero-length subslice. Otherwise borrow the range `1..len - 1`. Sum in a loop with an `i32` accumulator. Borrow the mutable `start..end` region before calling `sort`.

</details>
<details><summary>Check your predictions (after answering them)</summary>

- **P1 is `(2, 1)`.** A slice reference carries an address and a length. A reference to `[i32; 4]` carries only an address, because the length is part of the type.
- **P2 is `(2, 2, 0)`.** `&v[1..3]` covers indexes 1 and 2, so it starts at the value 2. Taking a subslice allocates nothing.
- **P3 is `[3, 5, 1, 4]`.** Only the first two elements are reversed.
- **P4 is `(3, 4)`.** `chunks(2)` gives pieces that do not overlap: two full ones and a last one with a single element. `windows(2)` gives every pair of neighbours.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`[i32]` is a run of i32 values of some length, with no size known at compile time, so you use it through a pointer. `[i32; 4]` is a fixed-size array value, and `&[i32]` is a fat reference: a pointer plus a length. Arrays, vectors, and subranges of either all coerce to `&[i32]`, so a slice parameter accepts more callers than `&Vec<i32>` and promises not to resize. `sort` and many other methods are defined on slices, and both arrays and vectors provide them by borrowing their elements as a slice. Taking a subslice copies nothing: it is a new pointer and length into the same storage.

</details>

## 10 · Unicode and memory

<details><summary>Hint 1</summary>

String lengths and string indices count bytes. `chars` walks Unicode scalar values; it does not group combining marks into user-perceived characters.

</details>
<details><summary>Hint 2</summary>

Use `len`, `chars().count()`, `chars().next()`, `size_of_val`, and `len_utf8`. Check `is_char_boundary(mid)` before using `split_at(mid)`; it also rejects an index beyond the end.

</details>
<details><summary>Check your predictions (after answering them)</summary>

- **P1 is `(7, 5)`.** `ü` and `ß` take two bytes each in UTF-8; the other three letters take one.
- **P2 is `(false, true)`.** 0xD800 is in the range reserved for UTF-16 and is not a scalar value. 0x1F980 is the crab emoji.
- **P3 is `[0, 2, 6]`.** `ñ` occupies bytes 0 and 1, and the crab occupies bytes 2 to 5. The end of the string is a boundary too.
- **P4 is `false`.** `==` on strings compares bytes. "e" followed by a combining accent is two scalars; the single precomposed letter is one. They look the same and are not equal.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A `char` is one Unicode scalar value, stored in 4 bytes, but a user-perceived character can be several scalars, such as "e" plus a combining accent, and `==` compares scalars without normalizing. A `String` is a pointer, capacity, and length that owns a heap buffer of UTF-8 bytes; an `&str` is a pointer and length that borrows UTF-8 bytes from somewhere. Lengths and string indices count bytes, so a slice must start and end on char boundaries or it panics. String literals are stored in the program's read-only data and need no heap allocation at run time.

</details>

## 11 · Strings and literals

<details><summary>Hint 1</summary>

Borrow input text with `&str`, create output with `String`, and use string methods to maintain valid UTF-8. A raw literal treats backslashes literally; a hash delimiter lets it include quotes.

</details>
<details><summary>Hint 2</summary>

Use `format!` for the message; `push_str` and `push` for the tag. For the list, make a `Vec` of `&str`, push `item.trim()` for each item in a loop, then call `join(" | ")` on the vector. `Vec::with_capacity(items.len())` saves the vector from growing, but `Vec::new()` passes too; what the test rejects is a `String` for every item. `String::pop` removes a complete Unicode scalar. The literal task can use `r#"..."#` as its raw form.

</details>
<details><summary>Check your predictions (after answering them)</summary>

- **P1 is `(2, 1)`.** In a raw literal, `\n` is two characters: a backslash and an `n`. In an ordinary literal it is one newline character.
- **P2 is the seven characters `"tab\t"`, quotes included.** Debug formatting prints a string as a literal: it adds the quotes and writes the tab as the escape `\t`. As Rust source, the answer is `"\"tab\\t\"".to_string()`.
- **P3 is `4`.** Two commas in a row have an empty piece between them: `"a"`, `"b"`, `""`, `"c"`.
- **P4 is `true`.** `+` takes the left `String` by value and appends to it. The buffer had room, so it was not reallocated.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Accepting `&str` lets callers pass string literals, `String`s, and slices of either without giving up ownership; returning `String` hands the caller a new buffer it owns. The `String` owns its UTF-8 buffer; an `&str` only borrows bytes owned by something else. `let mut s = "hi"` makes the *binding* mutable, so `s` can point at different text, but the text itself is still borrowed and cannot grow; you need `String::from` or `to_string` for that. `push_str` and `push` append into the existing buffer, reallocating only when it is full.

</details>

## 12 · Boundary types and aliases

<details><summary>Hint 1</summary>

The correct type depends on the external representation. Paths and OS strings need not be valid Unicode. C strings must be terminated by a zero byte and cannot contain interior zero bytes.

</details>
<details><summary>Hint 2</summary>

Copy a `b"..."` byte literal into a vector for `PacketBytes` (`to_vec` does it; `\x89` writes a byte by its hexadecimal value). Use `to_path_buf`, `as_os_str`, and a `c"..."` literal for the remaining tasks. The alias and `Vec<u8>` are interchangeable types.

</details>
<details><summary>Check your predictions (after answering them)</summary>

- **P1 is `(2, 3)`.** A byte string holds exactly the bytes you wrote. A C string literal adds a final zero byte.
- **P2 is `true`.** An alias is a second name for the same type.
- **P3 is `(Some("gz"), Some("archive.tar"))`.** The extension is what follows the *last* dot; the stem is everything before that dot.
- **P4 is `(3, 5)`.** The invalid byte becomes one replacement character, U+FFFD, which takes three bytes in UTF-8: 1 + 3 + 1 = 5.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

User text is `String`/`&str`; arbitrary file contents are `Vec<u8>`/`&[u8]`; filesystem paths are `PathBuf`/`&Path`; environment-variable names and other OS-provided strings are `OsString`/`&OsStr`; arguments to C functions are `CString`/`&CStr`. In each pair, the first type owns its buffer and the second borrows it. Paths and OS strings need not be valid UTF-8, and C strings end at one terminal zero byte, so they cannot contain an interior zero. A type alias is just another name for the same type: it adds no checks, and values of the alias and the original type are interchangeable.

</details>

## 14 · Checked conversions and numeric edge cases

<details><summary>Hint 1</summary>

Every task is one short expression built on a method from the standard library. Ask of each one: which value does not fit, and should the caller hear about it? `try_from` and `parse` return a `Result`; the `checked_` methods return an `Option`; a float-to-integer `as` cast never fails but stops at the limits of the target type.

</details>
<details><summary>Hint 2</summary>

A: `u16::try_from(value)` gives a `Result`; `.ok()` turns it into the `Option` you must return. B: multiply, call `round()`, then `as u8`. C: `unsigned_abs`. D: `checked_div` covers both failures. E: `rem_euclid(24)`; its result is between 0 and 23, so a cast to `u8` loses nothing. F: `readings.sort_by(f64::total_cmp)`. G and H: `to_be_bytes` and `u32::from_le_bytes`. I: `text.trim().parse()`; the function's return type tells `parse` what to produce.

</details>
<details><summary>Check your predictions (after answering them)</summary>

- **P1 is `(255, 0, 0, -1)`.** A float-to-integer cast drops the fraction, stops at the limits of the target type, and turns NaN into 0. 300.7 is above `u8::MAX`; −5.9 is below 0; −1.9 loses its fraction and becomes −1.
- **P2 is `(-1, 2, -2, -3)`.** `/` rounds toward zero and `%` takes the sign of the left operand: −7 = (−2) × 3 + (−1). The Euclidean pair keeps the remainder non-negative: −7 = (−3) × 3 + 2.
- **P3 is `([0x12, 0x34], 0x3412)`.** Big-endian writes the most significant byte first. Read as little-endian, the same two bytes put 0x12 in the low position.
- **P4 is `(true, false, false, true)`.** A leading `+` is accepted. A leading space is not. 256 does not fit in a `u8`. Floats accept exponent notation.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`as` can lose information silently: it drops bits, drops fractions, or stops at a limit, and gives no sign that it did. `From` only exists for pairs of types where every value fits, so it can never lose anything. `TryFrom` works for any pair of integer types and reports a value that does not fit as an `Err`. A multi-byte integer has no single "natural" byte sequence: processors disagree on which byte comes first, so a format must say big-endian or little-endian, and both sides must use that order. Floats cannot be sorted with `sort` because NaN is neither less than, equal to, nor greater than any value; `total_cmp` defines a complete order that places every value, including both zeros and NaN.

</details>

## 13 · Capstone

<details><summary>Hint 1</summary>

Solve each total independently before assembling the report. The checked total must preserve `None` on overflow, while the saturated total clamps at 255. The sorted data must be an owned copy.

</details>
<details><summary>Hint 2</summary>

For the checked helper, start a `u16` accumulator at zero and loop over the readings. Widen each reading with `u16::from(reading)` and add it with `checked_add`, which returns an `Option`. Look inside it with `match`: on `Some(sum)` store the new total, on `None` leave the function with `return None;`. After the loop, return `Some(total)`. (When you have studied chapter 7, the whole `match` shortens to `total = total.checked_add(u16::from(reading))?;`.) For the other helper, use `saturating_add` on a `u8` accumulator. In the report, trim the label, copy the readings with `to_vec`, sort the copy, and call your helpers. Measure the trimmed label, not the formatted message.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`to_vec` copies the readings into a new vector before sorting, so the input slice is untouched and the output owns its own buffer. Numeric conversions are needed where a u8 reading joins a u16 total (`u16::from` or a widening cast), and the totals use explicit policies (`checked_add`, `saturating_add`) so the result does not depend on the build profile. The `Summary` alias names the tuple type; it is still the same tuple. The compiler guarantees every `&str` is valid UTF-8 and outlives the call, so trimming and counting scalars cannot produce invalid text.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are two bugs: one is an accumulator that is too narrow (run the tests in both profiles and compare the failures), and one assumes every character is one byte. The clone in the third function is unnecessary.

</details>
<details><summary>Hint 2</summary>

`mean`: sum into a `u64` (a `u32` is still too small for 70,000 maximal readings) and divide by `readings.len() as u64`. The mean always fits back into a `u16`; `u16::try_from(mean).ok()` says so without a silent cast. `initials`: take the first character of each word with `word.chars().next()`, which returns an `Option<char>`. `first.to_uppercase()` yields one or more chars; append each to the output with a `for` loop and `push`, or all at once with `out.extend(first.to_uppercase())`. Do not create a `String` per word. `checksum`: keep a `u8` total and loop `for &byte in bytes { sum = sum.wrapping_add(byte); }`. Accept `&[u16]` and `&[u8]`.

</details>
<details><summary>Check your review</summary>

`mean` sums into a `u16`, which overflows for two readings of 60_000: debug builds panic and release builds silently wrap to a wrong answer. It also truncates `len()` with `as u16`: 70,000 readings become a divisor of 4,464, and exactly 65,536 readings become a divisor of 0, which panics with "attempt to divide by zero" in every profile. `initials` slices `&word[0..1]`, a byte range, so it panics on any word that starts with a non-ASCII letter, and it allocates a new String per word with `to_uppercase()` on a `&str`. `checksum` clones the whole vector just to read it. Its `u32` total is also a real overflow hazard, not only a matter of style: above about 16.8 million bytes of `0xFF` the sum passes `u32::MAX`, so a debug build panics; a release build happens to return the right checksum, because wrapping at 2^32 does not change a value modulo 256. A `u8` total with `wrapping_add` states the intent and behaves the same in both profiles. Clippy also flags `&Vec<_>` parameters, `len() == 0`, the index loop, and `sum = sum + ...`.

</details>

## Drills

Run `cargo drill ch03`. The compiler output explains *what* happened; these notes explain *why*. Read one only after the drill's verdict. Fixed versions of every drill are in [`solutions/drills/ch03/`](../../solutions/drills/ch03/).

<details><summary>d01 · Calling a method on an untyped literal</summary>E0689. Method lookup needs the receiver's concrete type, and `(-17)` is still "some integer" at that point; the later `17_i32` comes too late. Write `(-17_i32).abs()` or `i32::abs(-17)`.</details>
<details><summary>d02 · No implicit widening</summary>E0308. Rust never converts between numeric types implicitly, even when no information can be lost. Use `u32::from(small)` (lossless, preferred) or `small as u32`.</details>
<details><summary>d03 · Plain addition overflows</summary>Panics at run time ("attempt to add with overflow"), because drills are debug builds; a release build would silently wrap to 4. Choose the policy: `level.saturating_add(amount)`.</details>
<details><summary>d04 · Dividing a float by zero</summary>Compiles and runs. IEEE 754 defines `1.0 / 0.0` as positive infinity and `0.0 / 0.0` as NaN; only integer division by zero panics.</details>
<details><summary>d05 · An integer is not a condition</summary>E0308: an `if` condition must be `bool`, and integers are never truthy. Write the comparison you mean: `if pending != 0`.</details>
<details><summary>d06 · Indexing a tuple with a variable</summary>E0608: tuples have no run-time indexing, because each field may have a different type. Name the field: `reading.0`.</details>
<details><summary>d07 · Array lengths are part of the type</summary>E0435: `[0_u8; n]` needs `n` at compile time, and a parameter is a run-time value. Use `vec![0_u8; n]` for a run-time length, or a `const` length.</details>
<details><summary>d08 · Indexing a String</summary>E0277: `String` and `str` cannot be indexed by an integer, because a byte offset is not a character. The first scalar is `word.chars().next()`, which returns an `Option<char>` because the string may be empty. The KEEP line compares with a plain `char`, so take the value out: `word.chars().next().expect("word is not empty")`. (`word.as_bytes()[0]` gives the first <em>byte</em> as a `u8`; that is a different question and does not fit the KEEP line.)</details>
<details><summary>d09 · Slicing inside a character</summary>Panics at run time: "ñ" is two bytes, so byte 1 is not a char boundary. Slice `&word[..2]` (or `&word[..'ñ'.len_utf8()]`), or find the end with `word.char_indices().nth(1)`.</details>
<details><summary>d10 · Casting an integer to char</summary>E0604: only `u8` can be cast to `char`, because every u8 is a valid scalar but not every u32 is (surrogates, values above 0x10FFFF). `char::from_u32(code)` returns an `Option<char>`; the KEEP line needs a plain `char`, so write `char::from_u32(code).expect("valid scalar value")`.</details>

## Mastery check: model answers

Answer the thirteen questions in the [chapter guide](README.md#mastery-check) from memory first. Then compare.

<details><summary>Model answers (after writing your own)</summary>

1. **Required and inferred types.** A function's parameter types and return type must be written, and so must the type of a `const` or `static`. Inside a function body, the compiler infers the types of local variables from literals, annotations, the signature and later uses. A literal with no clue becomes `i32` or `f64`.
2. **Ranges, indexes, `as`.** `u8` holds 0 to 255 and `i8` holds −128 to 127. Indexes are `usize` because that type is as wide as a memory address, so it can count every element that fits in memory. `as` does not always keep the value: narrowing drops the high bits, a same-width signed/unsigned cast reads the same bits differently, and a float-to-integer cast drops the fraction and stops at the target's limits.
3. **Overflow APIs.** A pixel intensity should saturate (`saturating_add`): too bright stays at the maximum. A cyclic counter should wrap (`wrapping_add`). A size computation should be checked (`checked_mul`, `checked_add`) and the caller should handle `None`.
4. **NaN, signed zero, approximate equality.** NaN is not equal to itself and makes every ordering comparison false, so it breaks `==`, `<` and sorting unless you handle it. `0.0` and `-0.0` are equal but behave differently, for example as divisors. Most decimal fractions are rounded when stored and after each operation, so computed values should be compared with a tolerance.
5. **Tuple, array, unit.** A tuple has a fixed number of fields that may have different types, reached by position (`t.0`). An array has N elements of one type and can be indexed with a run-time value. `()` is the tuple with no fields: one possible value, no information.
6. **Who cleans up.** Nothing is freed through a `&T`; the owner elsewhere does that. A `Box<T>` frees its value when the box is dropped. `Rc<T>` and `Arc<T>` free the value when the last pointer to it is dropped; `Arc` counts atomically and may cross threads. A raw pointer has no owner and no checks: it may be null or dangling, and reading through it needs `unsafe`.
7. **Array, Vec, slice.** `[T; N]` stores N elements inline and owns them. `Vec<T>` stores a pointer, a capacity and a length, and owns a heap buffer of elements. `&[T]` stores a pointer and a length and borrows elements owned by something else.
8. **`&[T]` over `&Vec<T>`.** A slice parameter accepts arrays, vectors and parts of either, and it promises that the function will not change the length. `&Vec<T>` rejects everything except a whole vector and gives the function nothing more.
9. **String length and characters.** `len()` counts UTF-8 bytes. A `char` is one Unicode scalar value (4 bytes in memory, 1 to 4 bytes in UTF-8). A user-perceived character can consist of several scalars, such as a letter and a combining accent.
10. **`String` and `&str`.** A `String` owns a growable heap buffer of UTF-8. A `&str` is a pointer and a length that borrows UTF-8 bytes from a `String` or from a literal. Literal text is stored in the compiled program's read-only data and stays valid for the whole run.
11. **Other owned types.** `PathBuf` for file-system paths; `OsString` for strings that come from the operating system (environment variables, arguments, file names), which may not be valid Unicode; `Vec<u8>` for arbitrary bytes such as file contents; `CString` for text passed to C code, which must end with a zero byte.
12. **Aliases.** No. A type alias is another name for the same type. Values of the alias and of the original type are interchangeable, and the alias adds no validation.
13. **Conversions.** `From` exists only for pairs of types where every value fits, so the question cannot arise. `TryFrom` returns `Err` for a value that does not fit. `as` always produces a value: an integer loses its high bits, a float that is too large becomes the integer type's maximum, and NaN becomes 0. `%` gives a negative result when its left operand is negative (`-1 % 24` is `-1`), and a clock needs a result from 0 to 23; `rem_euclid` gives that.

</details>
