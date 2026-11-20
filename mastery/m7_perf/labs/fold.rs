//! Lab 1: what the optimizer does with a known input. See LABS.md.
//!
//! Build: rustc --edition 2024 -O --crate-type=lib --emit=llvm-ir -o fold.ll fold.rs

use std::hint::black_box;

#[inline]
pub fn digit_sum(mut n: u64) -> u32 {
    let mut sum = 0;
    while n > 0 {
        sum += (n % 10) as u32;
        n /= 10;
    }
    sum
}

/// The input is a constant that the compiler can see.
///
/// `#[inline(never)]` keeps this function in the output as a function of its
/// own, so that you can find it by name.
#[inline(never)]
pub fn known_input() -> u32 {
    digit_sum(1_234_567_890_123_456_789)
}

/// The same input, hidden behind `black_box`.
#[inline(never)]
pub fn opaque_input() -> u32 {
    digit_sum(black_box(1_234_567_890_123_456_789))
}
