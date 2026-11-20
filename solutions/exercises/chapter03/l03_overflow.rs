//! LESSON 03 — Choosing an overflow policy
//! Run: cargo test --test chapter03 l03_
//! Then: cargo test --release --test chapter03 l03_
//! Read first: CONCEPTS.md, section "Lesson 03".
//!
//! PREDICT: first answer `p_predictions` at the bottom (replace each `guess()`
//! with your answer). Then consider: For 250_u8 plus 10, write the result of
//! wrapping, checked, saturating, and overflowing addition. What does strict
//! addition do? What does ordinary addition do by default in debug and release
//! builds?
//!
//! EXPLAIN: Pick policies for image brightness, a cyclic counter, and a buffer
//! size that must reject overflow. Does a checked shift detect bits shifted out,
//! or an invalid shift distance? Why is casting AFTER addition too late?
//!
//! COMPILER LAB: put `std::hint::black_box(250_u8) + 10` in the scratch program
//! and print it. Compare normal and `--release` runs. black_box keeps this a
//! runtime operation for the experiment. Repair it with an explicit policy.

/// A. Return (wrapping, checked, saturating, overflowing) addition results.
/// Each field must use its corresponding method, with profile-independent behavior.
fn addition_policies(a: u8, b: u8) -> (u8, Option<u8>, u8, (u8, bool)) {
    (
        a.wrapping_add(b),
        a.checked_add(b),
        a.saturating_add(b),
        a.overflowing_add(b),
    )
}

/// B. Multiply using the strict method: overflow must panic in EVERY profile.
fn strict_product(a: u16, b: u16) -> u16 {
    a.strict_mul(b)
}

/// C. Average any two i32 values without overflow, rounding toward zero.
/// Use a wider intermediate or the built-in midpoint method.
fn midpoint(a: i32, b: i32) -> i32 {
    // `midpoint` on signed integers rounds toward zero and cannot overflow.
    // The same result by hand: ((a as i64 + b as i64) / 2) as i32.
    a.midpoint(b)
}

/// D. Return (checked shift-left, wrapping shift-left).
/// The policy applies to the DISTANCE; this is not checked multiplication.
fn left_shift_policies(value: u8, distance: u32) -> (Option<u8>, u8) {
    (value.checked_shl(distance), value.wrapping_shl(distance))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_safe_and_overflowing_addition() {
        assert_eq!(addition_policies(10, 20), (30, Some(30), 30, (30, false)));
        assert_eq!(addition_policies(250, 10), (4, None, 255, (4, true)));
        assert_eq!(
            addition_policies(255, 0),
            (255, Some(255), 255, (255, false))
        );
    }

    #[test]
    fn b_strict_product_still_multiplies_safe_inputs() {
        assert_eq!(strict_product(200, 300), 60_000);
        assert_eq!(strict_product(0, u16::MAX), 0);
    }

    #[test]
    #[should_panic(expected = "attempt to multiply with overflow")]
    fn b_strict_product_panics_on_overflow() {
        strict_product(500, 500);
    }

    #[test]
    fn c_midpoint_handles_both_extremes() {
        assert_eq!(midpoint(4, 10), 7);
        assert_eq!(midpoint(7, i32::MAX), 1_073_741_827);
        assert_eq!(midpoint(i32::MIN, i32::MAX), 0);
        assert_eq!(midpoint(i32::MIN, i32::MIN), i32::MIN);
        assert_eq!(midpoint(-4, -1), -2);
    }

    #[test]
    fn d_shift_checks_the_distance() {
        assert_eq!(left_shift_policies(3, 1), (Some(6), 6));
        assert_eq!(left_shift_policies(128, 1), (Some(0), 0));
        assert_eq!(left_shift_policies(3, 8), (None, 3));
        assert_eq!(left_shift_policies(3, 9), (None, 6));
        assert_eq!(left_shift_policies(0, 8), (None, 0));
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Wrapping addition on an unsigned type and on a signed type.
        let p1: (u8, i8) = (44, -56);
        assert_eq!(
            p1,
            (200_u8.wrapping_add(100), 100_i8.wrapping_add(100)),
            "P1: wrapping_add. Left = your prediction, right = what Rust computes."
        );

        // P2. i8::MIN has no positive counterpart. Negate it three ways.
        let p2: (Option<i8>, i8, i8) = (None, -128, 127);
        assert_eq!(
            p2,
            (
                i8::MIN.checked_neg(),
                i8::MIN.wrapping_neg(),
                i8::MIN.saturating_neg()
            ),
            "P2: negating the minimum. Left = your prediction, right = what Rust computes."
        );

        // P3. A checked shift that pushes one bits out of the top, and a
        // wrapping shift by more than the type's width.
        let p3: (Option<u8>, u32) = (Some(240), 2);
        assert_eq!(
            p3,
            (255_u8.checked_shl(4), 1_u32.wrapping_shl(33)),
            "P3: checked_shl and wrapping_shl. Left = your prediction, right = what Rust computes."
        );

        // P4. Subtracting below zero, reported instead of panicking.
        let p4: (u8, bool) = (255, true);
        assert_eq!(
            p4,
            0_u8.overflowing_sub(1),
            "P4: overflowing_sub. Left = your prediction, right = what Rust computes."
        );
    }
}
