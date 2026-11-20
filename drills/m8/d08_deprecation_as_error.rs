//! DRILL M8-08 · A deprecation in a strict project
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m8`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: a deprecation is a warning. What does the attribute at the top of this file do with it?

// This project turns every deprecation warning into an error.
#![deny(deprecated)] // KEEP

/// Stands for version 1.4.0 of a library crate.
mod textkit {
    #[deprecated(since = "1.4.0", note = "use `word_count` instead")] // KEEP
    pub fn get_word_count(text: &str) -> usize { // KEEP
        word_count(text) // KEEP
    } // KEEP

    pub fn word_count(text: &str) -> usize {
        text.split_whitespace().count()
    }
}

fn main() {
    // A user's code, written for 1.3.0.
    assert_eq!(textkit::get_word_count("to be or not"), 4);
}
