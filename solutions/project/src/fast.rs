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
    let pattern = ONES * u64::from(needle);
    let (words, tail) = haystack.as_chunks::<8>();
    for (n, word) in words.iter().enumerate() {
        // `from_ne_bytes` is a plain load of the eight bytes. The test below
        // only asks whether *some* byte is zero, so byte order does not matter.
        let x = u64::from_ne_bytes(*word) ^ pattern;
        if has_zero_byte(x) {
            // The exact position comes from a plain scan of these 8 bytes.
            return word.iter().position(|&b| b == needle).map(|at| n * 8 + at);
        }
    }
    tail.iter()
        .position(|&b| b == needle)
        .map(|at| words.len() * 8 + at)
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
    assert!(
        byte.is_ascii(),
        "split_ascii needs an ASCII byte, got {byte:#x}"
    );
    let at = find_byte(s.as_bytes(), byte)?;
    // SAFETY:
    // - In bounds: `find_byte` returned the index of a byte of `s`, so
    //   `at < s.len()`. Then `..at` ends inside `s`, and `at + 1 <= s.len()`
    //   makes `at + 1..` a valid (possibly empty) range.
    // - Char boundaries: the byte at `at` equals `byte`, which is ASCII
    //   (< 0x80, checked above). In UTF-8, every byte of a multi-byte
    //   character is >= 0x80, so an ASCII byte is always a complete one-byte
    //   character. A character therefore starts at `at`, and the next one
    //   starts at `at + 1` (or the string ends there). Both cuts are char
    //   boundaries, and both halves are valid UTF-8.
    unsafe { Some((s.get_unchecked(..at), s.get_unchecked(at + 1..))) }
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
    let pattern = ONES * u64::from(needle);
    let mut i = 0;
    while i + 8 <= haystack.len() {
        // SAFETY:
        // - In bounds: the loop condition gives `i + 8 <= haystack.len()`, so
        //   `add(i)` stays inside the slice's allocation and the 8 bytes read
        //   are all bytes of `haystack`.
        // - Alignment: `read_unaligned` has no alignment requirement, which
        //   is needed because a `&[u8]` may start at any address.
        // - Validity: the bytes come from a `&[u8]`, so they are
        //   initialized, and every bit pattern is a valid `u64`.
        let word = unsafe { haystack.as_ptr().add(i).cast::<u64>().read_unaligned() };
        if has_zero_byte(word ^ pattern) {
            // This word holds the first match; the plain scan below finds it.
            break;
        }
        i += 8;
    }
    haystack[i..]
        .iter()
        .position(|&b| b == needle)
        .map(|at| i + at)
}

/// `0x01` in every byte: multiplied by a byte, it repeats that byte 8 times.
const ONES: u64 = 0x0101_0101_0101_0101;
/// The high bit of every byte.
const HIGHS: u64 = 0x8080_8080_8080_8080;

/// Whether at least one byte of `x` is zero.
///
/// Subtracting 1 from every byte borrows into the high bit only for a zero
/// byte or for a byte that is already >= 0x80; `& !x` removes the second
/// case. Borrows between bytes can set a wrong bit *above* a real zero
/// byte, never without one, so the yes/no answer is exact.
fn has_zero_byte(x: u64) -> bool {
    x.wrapping_sub(ONES) & !x & HIGHS != 0
}
