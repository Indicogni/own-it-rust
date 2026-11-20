//! Lesson 07, prediction P3: a program that is compiled as edition 2024 and
//! uses `old_lib.rs`, which is compiled as edition 2021.
//! You do not need to edit this file.

fn main() {
    // `r#gen` is the raw form of the name `gen`: the way to write a name that
    // is a keyword in this crate's edition.
    assert_eq!(old_lib::r#gen(3), 1);
}
