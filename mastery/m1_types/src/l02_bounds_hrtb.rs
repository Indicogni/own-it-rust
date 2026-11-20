//! LESSON 02 — Lifetime bounds and higher-ranked trait bounds
//! Run (in mastery/m1_types/): cargo test --no-fail-fast l02_
//! Read first: "Lesson 02" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `F: Fn(&str) -> &str` has no lifetime name in it. Which lifetime does the
//!    caller choose, and which does the function that calls `F` choose?
//! 2. Does `T: 'static` mean that a value of type `T` lives until the program
//!    ends?
//! 3. A method takes `&mut self` and returns `&str`. Which borrow does the
//!    result keep alive when the lifetimes are elided?
//!
//! IMPLEMENT the tasks below. Tasks B, C and D need a change to the signature
//! as well as a body; each task says which part is yours. Keep the supplied
//! tests and add one edge case.
//! EXPLAIN afterward: Write the bound of task A without the sugar, with an
//! explicit `for<'a>`. Explain why a caller of `pick_each` could not name that
//! lifetime in a generic parameter list, and what `T: 'a` asks of a type.
//!
//! COMPILER LAB (optional): In a scratch file write
//! `let trim = |text: &str| text.trim();` and read the error. Then pass the
//! same closure directly to `pick_each`. Explain the difference in your notes.

use std::fmt::Display;
use std::ops::Add;

/// A. Call `pick` on every line and collect what it returns.
/// The results borrow from `lines`, not from `pick`.
pub fn pick_each<F>(lines: &[String], pick: F) -> Vec<&str>
where
    F: Fn(&str) -> &str,
{
    todo!("Implement this task")
}

/// An amount of money in cents. It is deliberately neither `Copy` nor
/// `Clone`, and two amounts can only be added BY REFERENCE: `&a + &b`.
#[derive(Debug, Default, PartialEq)]
pub struct Cents(pub u64);

impl Add<&Cents> for &Cents {
    type Output = Cents;

    fn add(self, rhs: &Cents) -> Cents {
        Cents(self.0 + rhs.0)
    }
}

/// B. Add up all items, starting from `T::default()`. Do not copy or clone
/// an item: add references, as in `&total + item`.
///
/// Your part of the signature: the `where` clause is incomplete. Write the
/// bound that makes `&total + item` type-check, where `total` is a local
/// variable of this function. The function must then accept `u64`, `f64`,
/// `Cents`, and a caller's own type that can only be added by reference
/// (`tests/cases/l02/sum_by_ref_never_copies.rs`).
pub fn sum_by_ref<T>(items: &[T]) -> T
where
    T: Default,
{
    todo!("Implement this task")
}

/// Splits source text into words. It never copies the text.
pub struct Lexer<'src> {
    rest: &'src str,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Lexer { rest: source }
    }

    /// C. Return the next word (a run of non-whitespace characters), or `None`
    /// when only whitespace is left.
    ///
    /// Your part of the signature: the lifetime of the result. As written, a
    /// word keeps the lexer mutably borrowed, so a caller cannot hold two
    /// words at once (`tests/cases/l02/words_outlive_the_borrow.rs`).
    pub fn next_word(&mut self) -> Option<&str> {
        todo!("Implement this task")
    }
}

/// D. Box the value as a `dyn Display` that may hold borrows lasting `'a`.
///
/// Your part of the signature: the `where` clause is incomplete. Write the
/// body first, read the error, then add the missing bound.
pub fn erase<'a, T>(value: T) -> Box<dyn Display + 'a>
where
    T: Display,
{
    todo!("Implement this task")
}

/// E. Return a closure that removes `prefix` from the start of a line: it
/// returns the rest of the line, or `None` when the line does not start with
/// `prefix`. The closure keeps `prefix` and returns a part of ITS ARGUMENT.
pub fn strip_with(prefix: &str) -> impl for<'a> Fn(&'a str) -> Option<&'a str> {
    // A closure is already here so that the starter compiles. Its body, and
    // what it captures, are yours.
    |line| todo!("Implement this task")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::DropLog;

    #[test]
    fn a_picks_borrow_from_the_lines() {
        let lines = vec![String::from("  alpha "), String::from("beta")];
        let picked = pick_each(&lines, |line| line.trim());
        assert_eq!(picked, ["alpha", "beta"]);
        assert!(
            std::ptr::eq(picked[1], lines[1].as_str()),
            "pick_each must return what `pick` returned (a part of the line), not a copy"
        );
        let firsts = pick_each(&lines, |line| &line[..1]);
        assert_eq!(firsts, [" ", "b"]);
        assert!(pick_each(&[], |line| line).is_empty());
    }

    #[test]
    fn b_sums_numbers() {
        assert_eq!(sum_by_ref(&[3_u64, 9, 4]), 16);
        assert_eq!(sum_by_ref(&[0.5_f64, 0.25]), 0.75);
        assert_eq!(
            sum_by_ref::<u64>(&[]),
            0,
            "an empty slice gives T::default()"
        );
        let items = vec![1_u64, 2, 3];
        assert_eq!(sum_by_ref(&items), 6);
        assert_eq!(items.len(), 3, "the slice must still be usable");
    }

    #[test]
    fn c_words_in_order() {
        let mut lexer = Lexer::new("  let total =\t42 ");
        assert_eq!(lexer.next_word(), Some("let"));
        assert_eq!(lexer.next_word(), Some("total"));
        assert_eq!(lexer.next_word(), Some("="));
        assert_eq!(lexer.next_word(), Some("42"));
        assert_eq!(lexer.next_word(), None);
        assert_eq!(lexer.next_word(), None, "the end must be stable");
        assert_eq!(Lexer::new("").next_word(), None);
    }

    #[test]
    fn c_words_point_into_the_source() {
        let source = String::from("one two");
        let mut lexer = Lexer::new(&source);
        lexer.next_word();
        let two = lexer.next_word().expect("a second word");
        assert!(
            std::ptr::eq(two, &source[4..]),
            "next_word must return a slice of the source text, not a copy"
        );
    }

    #[test]
    fn d_erased_value_may_borrow() {
        let local = String::from("borrowed");
        let boxed = erase(&local);
        assert_eq!(boxed.to_string(), "borrowed");
        let owned = erase(42_u8);
        assert_eq!(owned.to_string(), "42");
    }

    #[test]
    fn e_closure_returns_part_of_its_argument() {
        let prefix = String::from("# ");
        let strip = strip_with(&prefix);
        let title = String::from("# Title");
        let kept = strip(&title);
        {
            // A second call with a shorter-lived line: only a higher-ranked
            // closure can serve both calls.
            let inner = String::from("# gone soon");
            assert_eq!(strip(&inner), Some("gone soon"));
            assert_eq!(strip("no marker"), None);
        }
        let kept = kept.expect("the title starts with the prefix");
        assert_eq!(kept, "Title");
        assert!(
            std::ptr::eq(kept, &title[2..]),
            "the closure must return a slice of the line it was given, not a copy"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. `keep` demands `T: 'static`. A value that is passed to it is
        // dropped when `keep` returns. Write the log entries in order.
        fn keep<T: 'static>(value: T) -> usize {
            size_of_val(&value)
        }
        let log = DropLog::new();
        keep(log.noisy("passed to keep"));
        log.note("after the call");
        let p1: Vec<&str> = guess();
        assert_eq!(
            p1,
            log.entries(),
            "P1: order of log entries; `T: 'static` is a bound on the type, not a promise \
             to live forever (left: your prediction, right: actual)"
        );

        // P2. Sizes in bytes on a 64-bit target: (the function item `str::trim`,
        // a function pointer of type `for<'a> fn(&'a str) -> &'a str`).
        let item = str::trim;
        let pointer: for<'a> fn(&'a str) -> &'a str = item;
        let p2: (usize, usize) = guess();
        assert_eq!(
            p2,
            (size_of_val(&item), size_of_val(&pointer)),
            "P2: bytes in (function item, function pointer) (left: your prediction, right: actual)"
        );

        // P3. One closure, two calls. Each call picks its own lifetime for the
        // argument. What does `kept` hold at the end?
        let first_word = strip_first_word;
        let outer = String::from("alpha beta");
        let kept = first_word(&outer);
        {
            let inner = String::from("gamma delta");
            let _ = first_word(&inner);
        }
        let p3: &str = guess();
        assert_eq!(
            p3, kept,
            "P3: the value kept from the first call (left: your prediction, right: actual)"
        );
    }

    fn strip_first_word(text: &str) -> &str {
        text.split(' ').next().unwrap_or("")
    }
}
