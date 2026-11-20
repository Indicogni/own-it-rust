//! Lesson 07, prediction P3: a library that is compiled as edition 2021.
//! You do not need to edit this file.

/// `gen` is an ordinary name in edition 2021.
pub fn gen(seed: u32) -> u32 {
    seed * 7 % 10
}
