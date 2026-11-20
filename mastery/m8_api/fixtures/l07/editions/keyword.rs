//! Lesson 07, prediction P1. You do not need to edit this file.
//!
//! The same text is compiled twice: as edition 2021 and as edition 2024.

fn gen(seed: u32) -> u32 {
    seed * 7 % 10
}

fn main() {
    assert_eq!(gen(3), 1);
}
