//! LESSON 01 — Unary, binary, and assignment operators
//! Run: cargo test --test chapter12 l01_
//! Read first: "Lesson 01" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does a+b necessarily return the same type as
//! a? Does a+=b have an Output associated type?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why are assignment traits separate? What are the move semantics of
//! each receiver in these implementations?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch12_scratch.rs):
//! Implement a bit-mask newtype with BitAnd, BitOr, Not, and BitOrAssign in scratch; test 0
//! and all bits set. Try overloading &&, as, or = and identify why no corresponding trait
//! exists.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, and explanation are done.

use std::ops::{Add, AddAssign, Mul, Neg};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Delta {
    x: i32,
    y: i32,
}
impl Add for Delta {
    type Output = Self;
    /// A. Componentwise addition. Inputs are bounded so arithmetic fits.
    fn add(self, rhs: Self) -> Self {
        todo!("A: implement the documented contract")
    }
}
impl Neg for Delta {
    type Output = Self;
    /// B. Negate both coordinates.
    fn neg(self) -> Self {
        todo!("B: implement the documented contract")
    }
}
impl AddAssign for Delta {
    /// C. Mutate both coordinates with componentwise addition.
    fn add_assign(&mut self, rhs: Self) {
        todo!("C: implement the documented contract")
    }
}
impl Mul<i32> for Delta {
    type Output = Self;
    /// D. Scale both coordinates; the RHS type differs from Self.
    fn mul(self, rhs: i32) -> Self {
        todo!("D: implement the documented contract")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::run_op;
    use practice::{alloc::count, guess};
    use std::hint::black_box;

    // Every test runs your operator through `run_op` (see mod.rs). It returns
    // the closure's result, and fails with a message if an operator method
    // calls itself forever.
    #[test]
    fn a_addition_is_componentwise() {
        let sum = run_op("Delta + Delta", || {
            Delta { x: 2, y: -4 } + Delta { x: -2, y: 7 }
        });
        assert_eq!(sum, Delta { x: 0, y: 3 });
    }
    #[test]
    fn b_unary_negation_handles_signs() {
        let negated = run_op("-Delta", || -Delta { x: -3, y: 0 });
        assert_eq!(negated, Delta { x: 3, y: 0 });
    }
    #[test]
    fn c_assignment_changes_the_original() {
        let d = run_op("Delta += Delta", || {
            let mut d = Delta { x: 4, y: 9 };
            d += Delta { x: 1, y: -9 };
            d
        });
        assert_eq!(d, Delta { x: 5, y: 0 }, "`+=` must change the left operand");
    }
    #[test]
    #[allow(clippy::erasing_op)] // Multiplying by zero is a deliberate edge case.
    fn d_rhs_can_be_another_type() {
        let (scaled, zeroed) = run_op("Delta * i32", || {
            (Delta { x: 3, y: -2 } * 4, Delta { x: 1, y: 2 } * 0)
        });
        assert_eq!(scaled, Delta { x: 12, y: -8 });
        assert_eq!(zeroed, Delta { x: 0, y: 0 });
    }

    #[test]
    fn b_negation_flips_both_coordinates() {
        let (mixed, zero) = run_op("-Delta", || (-Delta { x: 5, y: -7 }, -Delta { x: 0, y: 0 }));
        assert_eq!(mixed, Delta { x: -5, y: 7 });
        assert_eq!(zero, Delta { x: 0, y: 0 });
    }

    #[test]
    fn d_negative_scale_flips_signs() {
        let scaled = run_op("Delta * i32", || Delta { x: -1, y: 2 } * -3);
        assert_eq!(scaled, Delta { x: 3, y: -6 });
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. `String + &str` takes the left String by value. Does the result reuse
        // its heap buffer?
        let mut s = String::with_capacity(16);
        s.push_str("ab");
        let before = s.as_ptr();
        let joined = s + "cd";
        let p1: bool = guess();
        assert_eq!(
            p1,
            joined.as_ptr() == before,
            "P1: is the result's buffer the left operand's buffer?"
        );
        assert_eq!(joined, "abcd");

        // P2. `!` is `Not`: logical for bool, bitwise for integers.
        let p2: (u8, i8, bool) = guess();
        assert_eq!(p2, (!0u8, !5i8, !true), "P2: `!` on u8, i8 and bool");

        // P3. Heap allocations made by the whole counted closure: one
        // `with_capacity(8)`, then four `+=` of two bytes each.
        let (text, allocations) = count(|| {
            let mut s = String::with_capacity(8);
            for _ in 0..4 {
                s += black_box("ab");
            }
            black_box(s)
        });
        let p3: usize = guess();
        assert_eq!(p3, allocations, "P3: allocations in the counted closure");
        assert_eq!(text, "abababab");
    }
}
