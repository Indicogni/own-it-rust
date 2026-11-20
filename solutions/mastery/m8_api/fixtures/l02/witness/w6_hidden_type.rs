//! WITNESS 6 — the body of `words` changes, its signature does not (change T8)
//!
//! The change to `fixtures/l02/courier.rs`:
//!
//!     pub fn words(text: &str) -> impl Iterator<Item = &str> {
//!   -     text.split_whitespace()
//!   +     let words: Box<dyn Iterator<Item = &str>> =
//!   +         Box::new(text.split(' ').filter(|word| !word.is_empty()));
//!   +     words
//!     }
//!
//! For text without tabs and line breaks, both bodies yield the same words.
//!
//! Write a program that is correct for courier 1.0.0 (it compiles, runs, and
//! its assertions hold) and that the compiler rejects when the library has
//! this change. Such a program is a witness: it proves that the change can
//! break a user. Then replace the `?` by the error code that you expect:
//!
//! EXPECT: E0277

fn main() {
    let words = courier::words("to be or not");
    // The signature promises only `Iterator`. The compiler still looks through
    // `impl Iterator` to decide whether the hidden type is `Send`.
    let count = std::thread::spawn(move || words.count())
        .join()
        .expect("the worker does not panic");
    assert_eq!(count, 4);
}
