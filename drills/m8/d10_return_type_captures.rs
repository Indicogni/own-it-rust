//! DRILL M8-10 · What `impl Trait` borrows in edition 2024
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m8`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: the returned iterator owns a vector. What does its type say about the lifetime of `text`?

/// Stands for a library crate that you maintain.
mod words {
    /// The length of every word. The iterator owns its data: nothing in it
    /// points into `text`.
    pub fn lengths(text: &str) -> impl Iterator<Item = usize> {
        let lengths: Vec<usize> = text.split_whitespace().map(str::len).collect();
        lengths.into_iter()
    }
}

fn main() {
    let lengths = {
        let text = String::from("to be or not"); // KEEP
        words::lengths(&text) // KEEP
    }; // KEEP
    assert_eq!(lengths.sum::<usize>(), 9); // KEEP
}
