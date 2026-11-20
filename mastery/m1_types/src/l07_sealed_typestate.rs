//! LESSON 07 — Sealed traits, typestate and const generics
//! Run (in mastery/m1_types/): cargo test --no-fail-fast l07_
//! Read first: "Lesson 07" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A method takes `self` by value and returns a value of another type.
//!    What can the caller still do with the old value afterwards?
//! 2. A public trait has a supertrait that lives in a private module. Who can
//!    name the public trait? Who can implement it?
//! 3. `Matrix<2, 3>` and `Matrix<3, 2>` are different types. Which mistakes
//!    does that turn into compile errors?
//!
//! IMPLEMENT the two contracts below. There is no skeleton: every type, trait
//! and impl is yours. The contracts name the public items, and the programs in
//! `tests/cases/l07/` use exactly those names. Read the cases before you
//! start: the ones that must NOT compile are as much a part of the contract as
//! the ones that must run. Add one case-like test of your own to your notes.
//! EXPLAIN afterward: For each program in `tests/cases/l07/` that must not
//! compile, say which part of your design rejects it. Then say what a caller
//! could do if `Stage` were not sealed.
//!
//! COMPILER LAB (optional): Write the envelope with a run-time check instead
//! (one struct, an `Option<String>` body, `send` returns `Result`). List what
//! the caller must now handle that the typestate version made impossible.
//!
//! CONTRACT A — `Envelope<S>`: a message that can only be sent when complete
//!
//! Stages, as public zero-sized types: `NeedsTo`, `NeedsBody`, `Ready`.
//!
//! - `Stage`: a public trait implemented by exactly these three types, with
//!   `const NAME: &'static str` ("needs-to", "needs-body", "ready"). It is
//!   sealed with a private supertrait, so no other crate can add a stage.
//! - `HasRecipient`: a public trait with `Stage` as its supertrait,
//!   implemented by `NeedsBody` and `Ready` only. Through `Stage` it is
//!   sealed too. Generic code may ask for `S: HasRecipient` alone and still
//!   use everything that needs `S: Stage`.
//! - `Envelope::new(from: &str) -> Envelope<NeedsTo>`: the only way to make
//!   an envelope.
//! - `to(self, address: &str) -> Envelope<NeedsBody>`: in stage `NeedsTo` only.
//! - `cc(self, address: &str) -> Self`: in every stage that `HasRecipient`,
//!   written once, not once per stage.
//! - `body(self, text: String) -> Envelope<Ready>`: in stage `NeedsBody` only.
//!   The `String` is moved into the envelope, not copied.
//! - `text(&self) -> &str`: in stage `Ready` only; the body.
//! - `send(self) -> String`: in stage `Ready` only. It returns
//!   `"<from> -> <to>: <body>"`; with cc addresses the recipients are joined
//!   with ", " in the order in which they were added:
//!   `"<from> -> <to>, <cc1>, <cc2>: <body>"`.
//! - `stage(&self) -> &'static str`: in every stage; returns `S::NAME`.
//! - Every `Envelope<S>` has the same size, whatever the stage.
//!
//! CONTRACT B — `Matrix<R, C>`: shapes checked by the compiler
//!
//! - `Matrix<const R: usize, const C: usize>` holds `R` rows of `C` values of
//!   type `i64`.
//! - `Matrix::from_rows(rows: [[i64; C]; R]) -> Self` and
//!   `rows(&self) -> &[[i64; C]; R]`.
//! - `transpose(&self) -> Matrix<C, R>`.
//! - `mul(&self, rhs: &Matrix<C, K>) -> Matrix<R, K>` for every `K`: the
//!   matrix product. A wrong inner dimension is a type error.
//! - `Matrix::<N, N>::identity()`: for square matrices only.

use std::marker::PhantomData;

// Your code for contracts A and B goes here.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::DropLog;

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. A door whose state is a type parameter.
        // (size of Open in bytes, is Door<Open> exactly as large as a String?)
        struct Open;
        struct Door<State> {
            _name: String,
            _state: PhantomData<State>,
        }
        let p1: (usize, bool) = guess();
        assert_eq!(
            p1,
            (
                size_of::<Open>(),
                size_of::<Door<Open>>() == size_of::<String>()
            ),
            "P1: the cost of a state marker (left: your prediction, right: actual)"
        );

        // P2. A transition moves the parcel from one type to the next.
        // Write the log entries in order.
        struct Pending {
            parcel: crate::support::Noisy,
        }
        struct Shipped {
            _parcel: crate::support::Noisy,
        }
        fn ship(pending: Pending) -> Shipped {
            Shipped {
                _parcel: pending.parcel,
            }
        }
        let log = DropLog::new();
        let pending = Pending {
            parcel: log.noisy("parcel dropped"),
        };
        let shipped = ship(pending);
        log.note("shipped");
        drop(shipped);
        log.note("end");
        let p2: Vec<&str> = guess();
        assert_eq!(
            p2,
            log.entries(),
            "P2: a transition moves the fields and drops nothing (left: your prediction, right: actual)"
        );

        // P3. Const generic parameters are inferred from array lengths.
        // (what `width` returns for this array, the size of [[i64; 3]; 2] in bytes)
        fn width<const R: usize, const C: usize>(_rows: &[[i64; C]; R]) -> usize {
            C
        }
        let p3: (usize, usize) = guess();
        assert_eq!(
            p3,
            (width(&[[1, 2, 3], [4, 5, 6]]), size_of::<[[i64; 3]; 2]>()),
            "P3: inferred const parameter and array size (left: your prediction, right: actual)"
        );
    }
}
