//! Optional milestone m23 (Chapters 23–24): a fast byte search, and when
//! `unsafe` is worth its cost.
//!
//! The scanner spends its time looking for single bytes (`=`, `"`, `\`,
//! spaces). Looking at eight bytes per step instead of one is a classic
//! speed-up, called SWAR ("SIMD within a register"): load eight bytes into
//! one `u64` and test all eight with a few arithmetic operations.
//!
//! The milestone has three steps, in this order:
//!
//! 1. [`find_byte`]: the word-at-a-time search in **safe** Rust. Since
//!    `slice::as_chunks` exists, loading eight bytes needs no `unsafe`.
//! 2. [`split_ascii`]: a function where `unsafe` skips a check you can prove
//!    is unnecessary. The proof is the exercise: the `// SAFETY:` comment.
//! 3. Optional: [`find_byte_unaligned`], the same search with raw pointer
//!    reads, the way it was written before `as_chunks`. Then **measure** all
//!    versions and decide which one the program should use.
//!
//! The rule this milestone teaches: write the safe version first, and keep
//! an `unsafe` version only when a measurement shows that it pays for the
//! proof you now have to maintain. Callers never see `unsafe` either way:
//! every function here is safe to call with any arguments.
//!
//! Check the unsafe code with Miri, a nightly tool that runs the tests in an
//! interpreter and detects undefined behavior such as out-of-bounds reads
//! (run from the `project/` directory):
//!
//! ```sh
//! rustup toolchain install nightly-2026-11-20 --component miri
//! cargo +nightly-2026-11-20 miri test --test m23_unsafe
//! ```
//!
//! Tests passing is not a proof of soundness, and Miri checks only the paths
//! the tests execute. Your written safety argument is part of the milestone.

/// m23 A. The index of the first `needle` in `haystack`; must equal
/// `haystack.iter().position(|&b| b == needle)` for every input.
///
/// Safe code only. `haystack.as_chunks::<8>()` splits the slice into a slice
/// of `[u8; 8]` arrays and a tail of fewer than 8 bytes. For each array,
/// build a word with `u64::from_ne_bytes`, XOR it with `needle` repeated in
/// every byte (`0x0101_0101_0101_0101 * needle as u64`), and test for a zero
/// byte with `x.wrapping_sub(0x0101..01) & !x & 0x8080..80 != 0`. When a
/// word contains a match, find its exact position with a plain loop over
/// those 8 bytes. Search the tail with a plain loop.
///
/// The `forbid` attribute makes the compiler reject `unsafe` inside this
/// function. Leave it in place.
#[forbid(unsafe_code)]
pub fn find_byte(haystack: &[u8], needle: u8) -> Option<usize> {
    todo!("m23 A: word-at-a-time search in safe code")
}

/// m23 B. Split `s` around the first `byte`: `(before, after)`, neither
/// containing that occurrence. Panics, with a message that contains
/// `ASCII`, if `byte` is not ASCII.
///
/// Use [`find_byte`], then `str::get_unchecked` for both halves, which skips
/// the bounds check and the `char`-boundary check that `&s[..i]` performs.
/// Explain in the `// SAFETY:` comment why both ranges are in bounds and
/// why an ASCII byte's position, and the position after it, are always
/// `char` boundaries.
pub fn split_ascii(s: &str, byte: u8) -> Option<(&str, &str)> {
    todo!("m23 B: unchecked split at a proven boundary")
}

/// m23 C (optional comparison). [`find_byte`] with raw pointer reads; must
/// return the same result for every input.
///
/// Loop while at least 8 bytes remain and load each word with
/// `haystack.as_ptr().add(i).cast::<u64>().read_unaligned()`; the rest is
/// the same as in [`find_byte`]. Write the `// SAFETY:` comment before you
/// run it: which condition keeps the read in bounds, and why may the
/// address be unaligned?
///
/// Its tests are marked `#[ignore]`; run them, and the measurement, with
/// the commands in `tests/m23_unsafe.rs`. Then decide which version
/// `scan.rs` should call, and write the reason next to the call.
pub fn find_byte_unaligned(haystack: &[u8], needle: u8) -> Option<usize> {
    todo!("m23 C: the same search with unaligned raw reads")
}
