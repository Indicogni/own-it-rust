//! LESSON 01 — Subtyping and variance
//! Run (in mastery/m1_types/): cargo test --no-fail-fast l01_
//! Read first: "Lesson 01" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `'static` outlives every lifetime. Which way does the conversion between
//!    `&'static str` and `&'a str` go, and what does it cost at run time?
//! 2. Why may a `Vec<&'static str>` be used as a `Vec<&'a str>`, when a
//!    `&mut Vec<&'static str>` may not be used as a `&mut Vec<&'a str>`?
//! 3. A function accepts `&'a str`. Is it safe to hand it out as a function
//!    that accepts only `&'static str`? And the other way round?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: State the variance of `&'a T`, `&'a mut T`, `Cell<T>` and
//! `fn(T)` in `'a` and in `T`. For one invariant case, describe the program
//! that would be accepted, and what it would do, if the type were covariant.
//!
//! COMPILER LAB (optional): In task B, first write `slot` as the whole body
//! and read the two `note:` lines of the error. Then, in a scratch file, define
//! the same struct with a plain `&'a str` field in place of the `Cell`, and see
//! that the same function body is accepted.

use std::cell::Cell;

/// Reads text. The lifetime is used behind a shared reference only.
pub struct Reader<'a> {
    pub text: &'a str,
}

/// Holds a reference that can be replaced through `&self`.
pub struct Slot<'a> {
    pub current: Cell<&'a str>,
}

/// Holds a function that consumes text. The lifetime is used in an argument
/// position only.
pub struct Sink<'a> {
    pub accept: fn(&'a str) -> usize,
}

/// A. Return the same reader, now typed with the shorter lifetime.
/// The compiler accepts this conversion by itself; find the shortest body.
pub fn shorten_reader<'long: 'short, 'short>(reader: Reader<'long>) -> Reader<'short> {
    todo!("Implement this task")
}

/// B. Return a slot that holds the same text, typed with the shorter lifetime.
/// Returning `slot` itself does not compile. Read the error, then build a new
/// `Slot` from the text inside the old one.
pub fn shorten_slot<'long: 'short, 'short>(slot: Slot<'long>) -> Slot<'short> {
    todo!("Implement this task")
}

/// C. Return the same sink, now typed with the LONGER lifetime: a function
/// that accepts short-lived text also accepts long-lived text.
pub fn lengthen_sink<'long: 'short, 'short>(sink: Sink<'short>) -> Sink<'long> {
    todo!("Implement this task")
}

/// D. Append `entry` to `log` and return the new number of entries.
///
/// The signature below is the one a hurried author writes: one lifetime for
/// everything. It compiles, and a single call works. Two calls in a row do not
/// (`tests/cases/l01/record_twice.rs`). Change the lifetimes in the signature,
/// and only the lifetimes, so that the log is borrowed just for the call. A log
/// must still refuse an entry that does not live long enough
/// (`tests/cases/l01/record_rejects_short_entry.rs`).
pub fn record<'a>(log: &'a mut Vec<&'a str>, entry: &'a str) -> usize {
    todo!("Implement this task")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;

    fn count_bytes(text: &str) -> usize {
        text.len()
    }

    #[test]
    fn a_reader_is_shortened_for_free() {
        let text: &'static str = "static text";
        let local = String::from("local");
        let short: Reader<'_> = shorten_reader(Reader { text });
        // Both readers now have the lifetime of `local`.
        let both = [short, Reader { text: &local }];
        assert!(
            std::ptr::eq(both[0].text, text),
            "shorten_reader must return the same reader (same text address), not a copy of the text"
        );
        assert_eq!(both[1].text, "local");
    }

    #[test]
    fn b_slot_keeps_its_text() {
        let text: &'static str = "kept";
        let local = String::from("replacement");
        let slot: Slot<'_> = shorten_slot(Slot {
            current: Cell::new(text),
        });
        assert!(
            std::ptr::eq(slot.current.get(), text),
            "the new slot must hold the same text (same address) as the old slot"
        );
        // The shortened slot accepts short-lived text. The old slot never sees it.
        slot.current.set(&local);
        assert_eq!(slot.current.get(), "replacement");
    }

    #[test]
    fn c_sink_accepts_longer_lived_text() {
        let sink: Sink<'static> = lengthen_sink(Sink {
            accept: count_bytes,
        });
        assert_eq!((sink.accept)("static text"), 11);
        let local = String::from("local text");
        let narrow: Sink<'_> = Sink {
            accept: count_bytes,
        };
        assert_eq!((narrow.accept)(&local), 10);
    }

    #[test]
    fn d_record_appends() {
        let first = String::from("first");
        let mut log = Vec::new();
        assert_eq!(record(&mut log, &first), 1);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. A `&'static str` is used where a shorter lifetime is expected.
        // Does the conversion change the pointer? Answer true if `long` and
        // `short` have the same address and length.
        let long: &'static str = "abc";
        let local = String::from("xyz");
        let pair: [&str; 2] = [long, &local];
        let short = pair[0];
        let p1: bool = guess();
        assert_eq!(
            p1,
            std::ptr::eq(long, short),
            "P1: same address after a lifetime conversion? (left: your prediction, right: actual)"
        );

        // P2. Sizes, counted in machine words (`size_of::<usize>()`):
        // (a `Cell<&str>`, a `fn(&str) -> usize` pointer).
        let word = size_of::<usize>();
        let p2: (usize, usize) = guess();
        assert_eq!(
            p2,
            (
                size_of::<Cell<&str>>() / word,
                size_of::<fn(&str) -> usize>() / word
            ),
            "P2: words in (Cell<&str>, fn(&str) -> usize) (left: your prediction, right: actual)"
        );

        // P3. A function that accepts any `&str` is stored as a function that
        // accepts only `&'static str`. Is it still the same function pointer?
        let general: for<'a> fn(&'a str) -> usize = count_bytes;
        let specific: fn(&'static str) -> usize = general;
        let p3: bool = guess();
        assert_eq!(
            p3,
            general as usize == specific as usize,
            "P3: same function address after the conversion? (left: your prediction, right: actual)"
        );
    }
}
