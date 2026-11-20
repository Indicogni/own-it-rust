//! LESSON 08 — Coherence, the orphan rule and blanket impls
//! Run (in mastery/m1_types/): cargo test --no-fail-fast l08_
//! Read first: "Lesson 08" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Two crates that do not know each other both depend on your crate. Which
//!    impls may each of them write so that they can never collide?
//! 2. `impl<T: Display> Describe for T` covers a great many types at once.
//!    Which impls does it forbid, in your crate and in your users' crates?
//! 3. `impl From<Level> for u8`: the trait is foreign and the type is foreign.
//!    Why is it allowed?
//!
//! IMPLEMENT the two contracts below. There is no skeleton. The programs in
//! `tests/cases/l08/` are written as a USER of your library would write them,
//! in another crate; that is the point of view from which coherence matters.
//! EXPLAIN afterward: State the orphan rule in one sentence, including the
//! part about uncovered type parameters. Then explain why this library gives
//! its users `Shown<T>` and not a blanket impl of `Describe` for every
//! `T: Display`.
//!
//! COMPILER LAB (optional): Add `impl<T: std::fmt::Display> Describe for T`
//! to your finished solution and read every error. Each one names an impl that
//! the blanket impl has made impossible. Remove it again afterward.
//!
//! CONTRACT A — `Describe`: a text form for log output
//!
//! - `pub trait Describe { fn describe(&self) -> String; }`
//! - Numbers and flags: `i64` describes itself in decimal ("-3"); `bool` as
//!   "yes" or "no".
//! - Text: `str` and `String` describe themselves in double quotes: `"abc"`.
//!   (No escaping is needed.)
//! - Containers of describable items: `Option<T>` is "none" or the inner
//!   description; `[T]`, `[T; N]` and `Vec<T>` are the item descriptions,
//!   joined with ", " and wrapped in square brackets: `[1, 2]`, `[]`.
//! - Pointers: `&T` and `Box<T>` describe what they point to, for every
//!   describable `T`, sized or not. So `&str`, `&[i64]` and
//!   `Box<dyn Describe>` are describable.
//! - Users can implement `Describe` for their own types, including types
//!   that also implement `Display`.
//! - `Shown<T>(pub T)`: a public wrapper that describes any `T: Display` by
//!   its `Display` text, without quotes. This is how a user opts in to
//!   "just use Display".
//! - `pub fn describe_all<D: Describe>(items: &[D]) -> String` joins the
//!   descriptions with "; ".
//!
//! CONTRACT B — `Level`: conversions for a type you own
//!
//! The enum is given below. Add impls so that a user can write:
//! - `u8::from(Level::Warn)` (2) and `let n: u8 = level.into();`
//! - `Level::try_from(1_u8)`, which is `Ok(Level::Info)`; numbers above 2
//!   give `Err(UnknownLevel(n))`, where `UnknownLevel` is a public tuple
//!   struct around the rejected `u8` that implements `Debug` and `PartialEq`;
//! - `level == 2_u8` and `2_u8 == level`, in both orders.

use std::fmt::Display;

/// How important a log line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Debug = 0,
    Info = 1,
    Warn = 2,
}

// Your code for contracts A and B goes here.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;

    trait Tag {
        fn tag(&self) -> &'static str;
    }
    impl Tag for u8 {
        fn tag(&self) -> &'static str {
            "u8"
        }
    }
    impl Tag for &u8 {
        fn tag(&self) -> &'static str {
            "&u8"
        }
    }
    impl Tag for str {
        fn tag(&self) -> &'static str {
            "str"
        }
    }
    impl Tag for String {
        fn tag(&self) -> &'static str {
            "String"
        }
    }

    fn tag_of<T: Tag + ?Sized>(value: &T) -> &'static str {
        value.tag()
    }

    struct Meter;
    impl Meter {
        fn read(&self) -> &'static str {
            "inherent"
        }
    }
    trait Read {
        fn read(&self) -> &'static str {
            "trait"
        }
    }
    impl Read for Meter {}

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. `Tag` is implemented for `u8` and, separately, for `&u8`.
        // Which impl does each call reach? Answer with the returned texts.
        let number = 5_u8;
        let reference = &number;
        #[allow(clippy::needless_borrow)]
        let actual = (
            number.tag(),
            reference.tag(),
            (&reference).tag(),
            tag_of(&reference),
        );
        let p1: (&str, &str, &str, &str) = guess();
        assert_eq!(
            p1, actual,
            "P1: which impl a method call selects (left: your prediction, right: actual)"
        );

        // P2. `Meter` has an inherent method `read` and a trait method `read`.
        let meter = Meter;
        let p2: (&str, &str) = guess();
        assert_eq!(
            p2,
            (meter.read(), Read::read(&meter)),
            "P2: inherent method or trait method (left: your prediction, right: actual)"
        );

        // P3. `Tag` is implemented for `String` and for `str`.
        let owned = String::from("text");
        #[allow(clippy::needless_borrow)]
        let actual = (owned.tag(), owned.as_str().tag(), (&owned).tag());
        let p3: (&str, &str, &str) = guess();
        assert_eq!(
            p3, actual,
            "P3: String or str (left: your prediction, right: actual)"
        );
    }
}
