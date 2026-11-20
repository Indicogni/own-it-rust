//! LESSON 02 — Nested modules and private implementation
//! Run: cargo test --test chapter08 l02_
//!
//! Read first: CONCEPTS.md, section "Lesson 02".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider: Can a parent module use a
//! private item of a child module? Can a child use private items of an
//! ancestor?
//!
//! IMPLEMENT the tasks below. Keep the tests and add one edge case.
//!
//! EXPLAIN afterward: Explain why a module boundary should expose operations
//! that preserve invariants instead of the raw representation.
//!
//! LAB (optional): LABS.md, Lab 2, "Privacy experiments in the scratch file".
//! The checked lab in label_lab/ (step 2) has you split modules into files.

mod tariff {
    mod rules {
        /// A. Private-to-tariff fee calculation: base 3 plus 2 per item.
        pub(super) fn fee(items: u16) -> u32 {
            // Widen first: 2 * u16::MAX does not fit in u16.
            3 + 2 * u32::from(items)
        }
    }
    /// B. Public API: free for zero items; otherwise delegate to rules::fee.
    pub fn cost(items: u16) -> u32 {
        if items == 0 { 0 } else { rules::fee(items) }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_public_boundary_uses_private_rules() {
        assert_eq!(tariff::cost(1), 5);
        assert_eq!(tariff::cost(4), 11);
        assert_eq!(tariff::cost(u16::MAX), 131073);
    }
    #[test]
    fn b_empty_order_is_free() {
        assert_eq!(tariff::cost(0), 0);
    }

    mod english {
        pub fn greet() -> &'static str {
            "hello"
        }
        pub fn farewell() -> &'static str {
            "bye"
        }
    }
    mod french {
        pub fn greet() -> &'static str {
            "bonjour"
        }
    }
    mod phrases {
        pub use super::english::*;
        pub use super::french::greet;
    }

    const LEVEL: &str = "tests";
    mod outer {
        pub const LEVEL: &str = "outer";
        pub mod inner {
            const LEVEL: &str = "inner";
            pub fn levels() -> (&'static str, &'static str) {
                (self::LEVEL, super::LEVEL)
            }
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. A glob import and an explicit import both offer `greet`.
        let p1: (&str, &str) = ("bonjour", "bye");
        assert_eq!(
            p1,
            (phrases::greet(), phrases::farewell()),
            "P1: explicit imports shadow glob imports"
        );

        // P2. What do self:: and super:: name inside outer::inner?
        let p2: (&str, &str) = ("inner", "outer");
        assert_eq!(p2, outer::inner::levels(), "P2: relative paths");
        assert_eq!(LEVEL, "tests");
    }
}
