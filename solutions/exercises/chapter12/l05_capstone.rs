//! LESSON 05 — Capstone: a small bit-set API
//! Run: cargo test --test chapter12 l05_
//! Read first: "Lesson 05" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: Predict the results of union, intersection, and complement on
//! 8-bit masks. Does complement require knowing a collection length?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why does Eq remain consistent with derived Hash here? Why is indexing
//! a mask less natural than asking contains(bit)?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch12_scratch.rs):
//! Add BitXor and BitAndAssign, test identities x^x=0 and x&all=x, then explain why Deref,
//! function-call syntax, and ? have different trait rules from ordinary arithmetic
//! operators.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, and explanation are done.

use std::ops::{BitAnd, BitOr, BitOrAssign, Not};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Flags(u8);
impl BitOr for Flags {
    type Output = Self;
    /// A. Union: a bit is set when it is set in either operand.
    fn bitor(self, rhs: Self) -> Self {
        Flags(self.0 | rhs.0)
    }
}
impl BitAnd for Flags {
    type Output = Self;
    /// B. Intersection: a bit is set when it is set in both operands.
    fn bitand(self, rhs: Self) -> Self {
        Flags(self.0 & rhs.0)
    }
}
impl Not for Flags {
    type Output = Self;
    /// C. Complement: flip all 8 bits.
    fn not(self) -> Self {
        Flags(!self.0)
    }
}
impl BitOrAssign for Flags {
    /// D. Union in place: add rhs's bits to self.
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}
impl Flags {
    /// E. Return false for bit indices >=8, otherwise test that bit.
    fn contains(self, bit: u32) -> bool {
        // Test the width first. `1u8 << bit` with `bit >= 8` panics in a debug
        // build; a release build uses only the low three bits of `bit`.
        bit < 8 && self.0 & (1 << bit) != 0
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::run_op;

    // The operators run through `run_op` (see mod.rs), which fails with a message
    // if an operator method calls itself forever, for example `self | rhs` inside
    // `bitor`.
    #[test]
    fn a_union_combines_bits() {
        let (disjoint, same) = run_op("Flags | Flags", || {
            (Flags(0b0101) | Flags(0b1010), Flags(5) | Flags(5))
        });
        assert_eq!(disjoint, Flags(15));
        assert_eq!(same, Flags(5));
    }
    #[test]
    fn b_intersection_keeps_shared_bits() {
        let (shared, with_all) = run_op("Flags & Flags", || {
            (Flags(0b1100) & Flags(0b1010), Flags(255) & Flags(17))
        });
        assert_eq!(shared, Flags(8));
        assert_eq!(with_all, Flags(17));
    }
    #[test]
    fn c_complement_is_eight_bits_wide() {
        let (of_none, twice, mixed) =
            run_op("!Flags", || (!Flags(0), !!Flags(19), !Flags(0b1010_0101)));
        assert_eq!(of_none, Flags(255));
        assert_eq!(twice, Flags(19), "complementing twice gives the value back");
        assert_eq!(mixed, Flags(0b0101_1010));
    }
    #[test]
    fn d_assignment_updates_the_receiver() {
        let f = run_op("Flags |= Flags", || {
            let mut f = Flags(1);
            f |= Flags(4);
            f |= Flags(4);
            f
        });
        assert_eq!(f, Flags(5), "`|=` must add the bits to the left operand");
    }
    #[test]
    fn e_membership_checks_the_width_before_shifting() {
        assert!(Flags(129).contains(0));
        assert!(Flags(129).contains(7));
        assert!(!Flags(129).contains(1));
        assert!(
            !Flags(255).contains(8),
            "there is no bit 8 in eight bits: answer false in debug and in release"
        );
        assert!(
            !Flags(255).contains(u32::MAX),
            "an index far outside the width is not a member either"
        );
    }
    #[test]
    fn e_membership_agrees_with_the_operators() {
        let all_agree = run_op("Flags operators with contains", || {
            let (a, b) = (Flags(0b0110_0001), Flags(0b0100_1001));
            (0..8).all(|bit| {
                (a | b).contains(bit) == (a.contains(bit) || b.contains(bit))
                    && (a & b).contains(bit) == (a.contains(bit) && b.contains(bit))
                    && (!a).contains(bit) != a.contains(bit)
            })
        });
        assert!(
            all_agree,
            "for every bit: union means `||`, intersection means `&&`, complement means `!`"
        );
    }
}
