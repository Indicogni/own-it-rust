//! LESSON 06 — Zero-sized types and uninhabited types
//! Run (in mastery/m1_types/): cargo test --no-fail-fast l06_
//! Read first: "Lesson 06" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A type with exactly one value needs no bits to store it. How many values
//!    do `()`, `[u64; 0]` and `PhantomData<String>` have? How many does
//!    `enum Never {}` have?
//! 2. `Result<T, E>` has room for either a `T` or an `E`. What is left of it
//!    when no value of type `E` can exist?
//! 3. A struct with a private field cannot be built outside its module. What
//!    does holding a value of such a type prove about the code that ran?
//!
//! IMPLEMENT the tasks below. A has a signature. B gives you a first attempt
//! to improve. C has no skeleton at all: design the type from the contract;
//! the checks are the programs in `tests/cases/l06/`. Add one edge case.
//! EXPLAIN afterward: Explain what "a value of this type proves something"
//! means for `AdminToken` and for `Result<T, Infallible>`, and why neither
//! proof costs anything at run time.
//!
//! COMPILER LAB (optional): In a scratch file write `let x: ! = panic!();`.
//! Read the error, then write a function `fn stop() -> ! { panic!() }`, which
//! is accepted. Note in your own words where the never type may be written on
//! this toolchain.
//!
//! NOTE (newer Rust versions): Rust 1.100 (November 2026) stabilised the never
//! type `!` and made `std::convert::Infallible` an alias of it. This course is
//! pinned to 1.99, where `let x: ! = panic!();` is error E0658 and
//! `Infallible` is an empty enum. On 1.100 and later that line compiles, so
//! the lab's answer depends on the toolchain you run it with.

use std::convert::Infallible;

/// A. Take the value out of a result that cannot be an error.
/// Do not use `unwrap`, `expect`, `panic!` or `unreachable!`: let the
/// compiler see for itself that no error exists.
pub fn unwrap_infallible<T>(result: Result<T, Infallible>) -> T {
    // `Infallible` has no values, so `Err(_)` cannot occur and the pattern
    // `Ok(value)` covers everything.
    let Ok(value) = result;
    value
}

/// Parsing from text, with an error type chosen by each implementor.
pub trait Parse: Sized {
    type Error;

    fn parse(text: &str) -> Result<Self, Self::Error>;
}

/// Text in upper case. Every input is acceptable.
#[derive(Debug, PartialEq)]
pub struct Shout(pub String);

/// An even number.
#[derive(Debug, PartialEq)]
pub struct Even(pub u32);

/// Why a text is not an `Even`.
#[derive(Debug, PartialEq)]
pub enum EvenError {
    NotANumber,
    Odd(u32),
}

/// B. `Shout::parse` upper-cases the text (ASCII) and cannot fail.
///
/// First attempt: the error type is `String`, and no caller can know that the
/// error never happens. Choose an error type that says so in the signature;
/// then a caller may write `let Ok(shout) = Shout::parse(text);`
/// (`tests/cases/l06/shout_cannot_fail.rs`).
impl Parse for Shout {
    type Error = Infallible;

    fn parse(text: &str) -> Result<Self, Self::Error> {
        Ok(Shout(text.to_ascii_uppercase()))
    }
}

/// B, second half. `Even::parse` accepts decimal text for an even `u32`
/// (surrounding whitespace is allowed) and reports the two ways to fail.
impl Parse for Even {
    type Error = EvenError;

    fn parse(text: &str) -> Result<Self, Self::Error> {
        let number: u32 = text.trim().parse().map_err(|_| EvenError::NotANumber)?;
        if number.is_multiple_of(2) {
            Ok(Even(number))
        } else {
            Err(EvenError::Odd(number))
        }
    }
}

// C. Design `AdminToken`, a proof that a password check succeeded.
//
// Contract:
// - `pub fn log_in(password: &str) -> Option<AdminToken>` returns a token
//   exactly when the password is "open sesame".
// - `pub fn wipe(proof: &AdminToken, data: &mut Vec<u8>)` empties `data`.
//   It needs no password check of its own: the token is the check.
// - An `AdminToken` is zero-sized.
// - Code outside this module cannot create an `AdminToken` in any way other
//   than `log_in`, and cannot duplicate one (no `Clone`, no `Copy`).
//
// Checked by `tests/cases/l06/token_is_free_proof.rs` and
// `tests/cases/l06/token_cannot_be_forged.rs`.
/// Proof that `log_in` succeeded. The private field keeps other modules from
/// writing `AdminToken { .. }` themselves.
pub struct AdminToken {
    _private: (),
}

pub fn log_in(password: &str) -> Option<AdminToken> {
    (password == "open sesame").then_some(AdminToken { _private: () })
}

pub fn wipe(_proof: &AdminToken, data: &mut Vec<u8>) {
    data.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::alloc::count;

    #[test]
    fn a_value_comes_out() {
        assert_eq!(unwrap_infallible(Ok::<_, Infallible>(7)), 7);
        let text = String::from("kept");
        let address = text.as_ptr();
        let out = unwrap_infallible(Ok(text));
        assert_eq!(
            out.as_ptr(),
            address,
            "the value must be moved out, not copied"
        );
    }

    #[test]
    fn b_shout_parses_everything() {
        let shout = match Shout::parse("quiet, please") {
            Ok(shout) => shout,
            Err(_) => panic!("Shout::parse must never return Err"),
        };
        assert_eq!(shout, Shout("QUIET, PLEASE".to_string()));
    }

    #[test]
    fn b_shout_error_is_uninhabited() {
        assert_eq!(
            size_of::<Result<Shout, <Shout as Parse>::Error>>(),
            size_of::<Shout>(),
            "with an error type that has no values, the Result needs no room for an error \
             and no tag: it must be exactly as large as a Shout"
        );
        assert_eq!(
            size_of::<Option<<Shout as Parse>::Error>>(),
            0,
            "an Option of a type without values can only be None, so it must be zero-sized"
        );
    }

    #[test]
    fn b_even_reports_both_failures() {
        assert_eq!(Even::parse(" 42 "), Ok(Even(42)));
        assert_eq!(Even::parse("0"), Ok(Even(0)));
        assert_eq!(Even::parse("7"), Err(EvenError::Odd(7)));
        assert_eq!(Even::parse("seven"), Err(EvenError::NotANumber));
        assert_eq!(Even::parse("-2"), Err(EvenError::NotANumber));
        assert_eq!(Even::parse(""), Err(EvenError::NotANumber));
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. In bytes, on a 64-bit target: (size of (), size of [u64; 0],
        // alignment of [u64; 0], size of (u8, [u64; 0])).
        let p1: (usize, usize, usize, usize) = (0, 0, 8, 8);
        assert_eq!(
            p1,
            (
                size_of::<()>(),
                size_of::<[u64; 0]>(),
                align_of::<[u64; 0]>(),
                size_of::<(u8, [u64; 0])>()
            ),
            "P1: zero-sized does not mean alignment 1 (left: your prediction, right: actual)"
        );

        // P2. In bytes: (Option<Infallible>, Result<u32, Infallible>,
        // Result<u32, ()>).
        let p2: (usize, usize, usize) = (0, 4, 8);
        assert_eq!(
            p2,
            (
                size_of::<Option<Infallible>>(),
                size_of::<Result<u32, Infallible>>(),
                size_of::<Result<u32, ()>>()
            ),
            "P2: sizes with an uninhabited or a zero-sized error (left: your prediction, right: actual)"
        );

        // P3. A thousand `()` are pushed into a vector.
        // (its length, the heap allocations made, is its capacity usize::MAX?)
        let (units, allocations) = count(|| {
            let mut units = Vec::new();
            for _ in 0..1000 {
                units.push(());
            }
            units
        });
        let p3: (usize, usize, bool) = (1000, 0, true);
        assert_eq!(
            p3,
            (units.len(), allocations, units.capacity() == usize::MAX),
            "P3: a Vec of zero-sized values (left: your prediction, right: actual)"
        );
    }
}
