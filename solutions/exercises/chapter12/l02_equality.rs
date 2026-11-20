//! LESSON 02 — Equivalence and NaN
//! Run: cargo test --test chapter12 l02_
//! Read first: "Lesson 02" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Is equality reflexive for f64? Should a
//! metadata field affect the identity of a record?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: List reflexivity, symmetry, and transitivity. Why must Eq not be
//! added to a wrapper whose equality follows NaN behavior?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch12_scratch.rs):
//! Attempt deriving Eq for a struct holding f64. Then create a case-insensitive ASCII name
//! newtype; explain why a future Hash implementation must use the same normalized identity.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, and explanation are done.

#[derive(Debug)]
struct Record {
    id: u64,
    note: String,
}
impl PartialEq for Record {
    /// A. Records are equal exactly when ids match; ignore note.
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Record {}
#[derive(Debug)]
struct Reading(f64);
impl PartialEq for Reading {
    /// B. Preserve f64 partial equality, including NaN != NaN.
    fn eq(&self, other: &Self) -> bool {
        // Delegating to f64 keeps its rules: NaN != NaN and 0.0 == -0.0.
        // That is also why `Reading` must not implement `Eq`.
        self.0 == other.0
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::run_op;

    fn record(id: u64, note: &str) -> Record {
        Record {
            id,
            note: note.into(),
        }
    }

    // The comparisons run through `run_op` (see mod.rs), which fails with a
    // message if `eq` calls itself forever, for example by using `self == other`.
    #[test]
    #[allow(clippy::eq_op)] // Comparing a value with itself is the point of the last check.
    fn a_record_identity_ignores_metadata() {
        let (same_id, other_id, itself) = run_op("Record == Record", || {
            let a = record(1, "old");
            (a == record(1, "new"), a == record(2, "old"), a == a)
        });
        assert!(
            same_id,
            "equal ids make equal records, whatever the notes say"
        );
        assert!(
            !other_id,
            "different ids make different records, even with equal notes"
        );
        assert!(itself, "a record must equal itself: Record claims Eq");
    }
    #[test]
    fn b_float_wrapper_keeps_nan_and_signed_zero_semantics() {
        let (nans, zeros, same, different) = run_op("Reading == Reading", || {
            (
                Reading(f64::NAN) == Reading(f64::NAN),
                Reading(0.0) == Reading(-0.0),
                Reading(4.0) == Reading(4.0),
                Reading(4.0) == Reading(5.0),
            )
        });
        assert!(
            !nans,
            "two NaN readings must compare like two NaN f64 values"
        );
        assert!(
            zeros,
            "0.0 and -0.0 must compare like the f64 values they wrap"
        );
        assert!(same, "4.0 equals 4.0");
        assert!(!different, "4.0 differs from 5.0");
    }

    #[test]
    fn a_not_equal_is_the_opposite_of_equal() {
        // `!=` is `PartialEq::ne`, which std derives from your `eq`.
        let (same_id, other_id) = run_op("Record != Record", || {
            (
                record(7, "x") != record(7, "y"),
                record(7, "x") != record(8, "x"),
            )
        });
        assert!(!same_id, "`!=` must be false when `==` is true");
        assert!(other_id, "`!=` must be true when `==` is false");
    }

    #[derive(PartialEq)]
    struct Point {
        x: f64,
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. NaN equality, directly and inside a collection.
        let (nan, also_nan) = (f64::NAN, f64::NAN);
        let p1: (bool, bool) = (false, false);
        assert_eq!(
            p1,
            (nan == also_nan, [1.0, nan].contains(&also_nan)),
            "P1: NaN with `==` and with `contains`"
        );

        // P2. Derived PartialEq compares fields with ==. Are the zeros equal? Are their
        // bit patterns equal?
        let p2: (bool, bool) = (true, false);
        assert_eq!(
            p2,
            (
                Point { x: 0.0 } == Point { x: -0.0 },
                0.0_f64.to_bits() == (-0.0_f64).to_bits()
            ),
            "P2: the two zeros, by `==` and by bits"
        );

        // P3. f64::max and f64::min with one NaN operand.
        let p3: (f64, f64) = (1.0, 1.0);
        assert_eq!(
            p3,
            (f64::NAN.max(1.0), f64::NAN.min(1.0)),
            "P3: max and min with a NaN operand"
        );
    }
}
