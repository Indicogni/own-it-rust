//! LESSON 14 — Checked conversions and numeric edge cases
//! Run: cargo test --test chapter03 l14_
//! Then: cargo test --release --test chapter03 l14_
//! Read first: CONCEPTS.md, section "Lesson 14".
//!
//! PREDICT: first answer `p_predictions` at the bottom (replace each `guess()`
//! with your answer). Then consider: What do `-1_i32 as u8` and
//! `u8::try_from(-1_i32)` produce? What is `300.7_f64 as u8`? Which i32 value
//! has no absolute value that fits in i32? Is `-7 % 3` positive? Can you call
//! `sort` on a slice of f64? Does `" 42".parse::<u8>()` succeed?
//!
//! EXPLAIN: `as`, `From` and `TryFrom` all convert numbers. Which one can lose
//! information silently, which one can never lose it, and which one reports
//! the loss? Why must a file format or a network protocol name a byte order?
//! Why does sorting floats need `total_cmp`?
//!
//! COMPILER LAB: try `let n: u8 = 300;`, then `let n = u8::from(300_i32);`,
//! then `let n: u8 = u8::try_from(300_i32);` in the scratch program. The three
//! diagnostics are different; read each one. Then call `sort()` on a `[f64; 3]`.

use std::num::ParseIntError;

/// A. Convert to a TCP port number. Return None when the value does not fit
/// in u16. Use `u16::try_from`; an `as` cast would silently wrap.
fn to_port(value: i64) -> Option<u16> {
    u16::try_from(value).ok()
}

/// B. Convert a brightness percentage to a level from 0 to 255: multiply by
/// 2.55, round to the nearest whole number, then cast straight to u8. Write no
/// `if`: rely on what a float-to-integer `as` cast does with values below 0,
/// values above 255, and NaN.
fn level_from_percent(percent: f64) -> u8 {
    // A float-to-integer `as` cast saturates at the target's limits and
    // turns NaN into 0, so no range check is needed here.
    (percent * 2.55).round() as u8
}

/// C. Distance of value from zero, as an unsigned number. It must be right
/// for every i32, including i32::MIN, in debug and release builds.
fn distance_from_zero(value: i32) -> u32 {
    value.unsigned_abs()
}

/// D. Divide a by b, rounding toward zero. Return None when the quotient does
/// not exist or does not fit in i32. There are two such cases; find both.
fn quotient(a: i32, b: i32) -> Option<i32> {
    // None for b == 0 and for i32::MIN / -1.
    a.checked_div(b)
}

/// E. A clock shows the hours 0 to 23. Return the hour it shows after moving
/// `offset` hours from midnight. The offset may be negative (back in time)
/// or longer than a day.
fn hour_of_day(offset: i32) -> u8 {
    // `rem_euclid(24)` is always in 0..24, so the cast cannot lose anything.
    offset.rem_euclid(24) as u8
}

/// F. Sort the readings in place from smallest to largest, using the total
/// order of `f64::total_cmp`: -0.0 comes before 0.0, and a NaN with a positive
/// sign comes after positive infinity. A NaN in the input must not panic.
fn sort_readings(readings: &mut [f64]) {
    readings.sort_by(f64::total_cmp);
}

/// G. Encode a frame length as four bytes in big-endian order (most
/// significant byte first), the usual order in network protocols.
fn length_prefix(length: u32) -> [u8; 4] {
    length.to_be_bytes()
}

/// H. Decode a u32 that a file format stores in little-endian order (least
/// significant byte first).
fn read_le(bytes: [u8; 4]) -> u32 {
    u32::from_le_bytes(bytes)
}

/// I. Parse a port number from text such as "8080". Whitespace around the
/// number is allowed, so trim first: `parse` itself rejects it. For anything
/// that is not a u16, return the error that `parse` reports.
fn parse_port(text: &str) -> Result<u16, ParseIntError> {
    // The return type tells `parse` to produce a u16.
    text.trim().parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::IntErrorKind;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[test]
    fn a_ports_outside_u16_are_rejected() {
        assert_eq!(to_port(0), Some(0));
        assert_eq!(to_port(8_080), Some(8_080));
        assert_eq!(to_port(65_535), Some(u16::MAX));
        assert_eq!(
            to_port(65_536),
            None,
            "65_536 does not fit in u16; an `as` cast would wrap it to 0"
        );
        assert_eq!(
            to_port(-1),
            None,
            "-1 is not a port; an `as` cast would turn it into 65_535"
        );
        assert_eq!(to_port(i64::MIN), None);
        assert_eq!(to_port(i64::MAX), None);
    }

    #[test]
    fn b_float_cast_rounds_first_then_clamps() {
        assert_eq!(level_from_percent(0.0), 0);
        assert_eq!(level_from_percent(20.0), 51);
        assert_eq!(
            level_from_percent(100.0),
            255,
            "round before casting: a bare cast drops the fraction"
        );
        assert_eq!(
            level_from_percent(250.0),
            255,
            "values above the range must clamp to 255; do not cast through a \
             wider integer first"
        );
        assert_eq!(level_from_percent(-10.0), 0, "values below 0 clamp to 0");
        assert_eq!(level_from_percent(f64::INFINITY), 255);
        assert_eq!(level_from_percent(f64::NEG_INFINITY), 0);
        assert_eq!(level_from_percent(f64::NAN), 0, "NaN casts to 0");
    }

    #[test]
    fn c_distance_handles_the_minimum() {
        assert_eq!(distance_from_zero(0), 0);
        assert_eq!(distance_from_zero(17), 17);
        assert_eq!(distance_from_zero(-17), 17);
        assert_eq!(distance_from_zero(i32::MAX), 2_147_483_647);
        let at_minimum = catch_unwind(|| distance_from_zero(i32::MIN));
        assert_eq!(
            at_minimum.ok(),
            Some(2_147_483_648),
            "i32::MIN is 2_147_483_648 away from zero. `abs` cannot return that \
             as an i32 (it panics in a debug build); use the method that \
             returns u32"
        );
    }

    #[test]
    fn d_quotient_rejects_both_failures() {
        assert_eq!(quotient(7, 2), Some(3));
        assert_eq!(quotient(-7, 2), Some(-3));
        assert_eq!(quotient(0, 5), Some(0));
        assert_eq!(quotient(i32::MIN, 1), Some(i32::MIN));
        assert_eq!(quotient(i32::MAX, -1), Some(-i32::MAX));
        let by_zero = catch_unwind(|| quotient(1, 0));
        assert_eq!(
            by_zero.ok(),
            Some(None),
            "dividing by zero must return None, not panic"
        );
        let overflow = catch_unwind(|| quotient(i32::MIN, -1));
        assert_eq!(
            overflow.ok(),
            Some(None),
            "i32::MIN / -1 would be 2_147_483_648, which does not fit in i32: \
             return None, not a panic"
        );
    }

    #[test]
    fn e_hours_stay_between_0_and_23() {
        assert_eq!(hour_of_day(0), 0);
        assert_eq!(hour_of_day(5), 5);
        assert_eq!(hour_of_day(24), 0);
        assert_eq!(hour_of_day(49), 1);
        assert_eq!(
            hour_of_day(-1),
            23,
            "one hour before midnight is 23; `%` gives -1 for a negative offset"
        );
        assert_eq!(hour_of_day(-24), 0);
        assert_eq!(hour_of_day(-25), 23);
        assert_eq!(hour_of_day(i32::MAX), 7);
        assert_eq!(hour_of_day(i32::MIN), 16);
    }

    #[test]
    fn f_sort_follows_the_total_order() {
        // `copysign` gives a NaN whose sign bit is clear on every platform.
        let nan = f64::NAN.copysign(1.0);
        let mut readings = [3.0, nan, 0.0, -0.0, f64::NEG_INFINITY, 1.5, f64::INFINITY];
        let outcome = catch_unwind(AssertUnwindSafe(|| sort_readings(&mut readings)));
        assert!(
            outcome.is_ok(),
            "sort_readings panicked on a NaN: `partial_cmp` has no answer for \
             NaN, `total_cmp` always has one"
        );
        // Debug formatting shows the sign of zero, so the order is readable.
        assert_eq!(
            format!("{readings:?}"),
            "[-inf, -0.0, 0.0, 1.5, 3.0, inf, NaN]",
            "the readings are not in `total_cmp` order"
        );
    }

    #[test]
    fn f_sort_handles_ordinary_and_empty_input() {
        let mut readings = [2.5, -1.0, 10.0, 0.25];
        sort_readings(&mut readings);
        assert_eq!(readings, [-1.0, 0.25, 2.5, 10.0]);
        let mut zeros = [0.0, -0.0, 0.0, -0.0];
        sort_readings(&mut zeros);
        assert_eq!(
            format!("{zeros:?}"),
            "[-0.0, -0.0, 0.0, 0.0]",
            "`total_cmp` puts -0.0 before 0.0, although `==` calls them equal"
        );
        sort_readings(&mut []);
    }

    #[test]
    fn g_length_prefix_is_big_endian() {
        assert_eq!(
            length_prefix(0x0102_0304),
            [1, 2, 3, 4],
            "big-endian puts the most significant byte first"
        );
        assert_eq!(length_prefix(258), [0, 0, 1, 2]);
        assert_eq!(length_prefix(0), [0; 4]);
        assert_eq!(length_prefix(u32::MAX), [0xff; 4]);
    }

    #[test]
    fn h_read_le_is_little_endian() {
        assert_eq!(
            read_le([4, 3, 2, 1]),
            0x0102_0304,
            "little-endian puts the least significant byte first"
        );
        assert_eq!(read_le([42, 0, 0, 0]), 42);
        assert_eq!(read_le([0, 0, 0, 0x80]), 2_147_483_648);
    }

    #[test]
    fn g_h_opposite_orders_mirror_each_other() {
        for length in [0, 1, 258, 0xdead_beef, u32::MAX] {
            let mut bytes = length_prefix(length);
            bytes.reverse();
            assert_eq!(
                read_le(bytes),
                length,
                "reversing big-endian bytes gives the little-endian bytes of the same number"
            );
        }
    }

    #[test]
    fn i_parse_accepts_ports_and_reports_errors() {
        assert_eq!(parse_port("8080"), Ok(8_080));
        assert_eq!(parse_port("0"), Ok(0));
        assert_eq!(parse_port("65535"), Ok(u16::MAX));
        assert_eq!(parse_port("+80"), Ok(80));
        assert_eq!(
            parse_port(" 443\n"),
            Ok(443),
            "trim the text before parsing; `parse` rejects whitespace"
        );
        for (text, kind) in [
            ("65536", IntErrorKind::PosOverflow),
            ("99999999999", IntErrorKind::PosOverflow),
            ("-1", IntErrorKind::InvalidDigit),
            ("80a", IntErrorKind::InvalidDigit),
            ("0x50", IntErrorKind::InvalidDigit),
            ("8 080", IntErrorKind::InvalidDigit),
            ("", IntErrorKind::Empty),
            ("   ", IntErrorKind::Empty),
        ] {
            let outcome = parse_port(text);
            assert_eq!(
                outcome.as_ref().map_err(|error| error.kind()),
                Err(&kind),
                "parse_port({text:?}) must fail with the error `parse::<u16>` \
                 reports; parsing into a wider type and casting hides it"
            );
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Four float-to-integer `as` casts. What does each one produce?
        // (`black_box` only hides the NaN from lints; `nan` is an ordinary NaN.)
        let above = 300.7_f64;
        let below = -5.9_f64;
        let small = -1.9_f64;
        let nan = std::hint::black_box(f64::NAN);
        let p1: (u8, u8, i32, i32) = (255, 0, 0, -1);
        assert_eq!(
            p1,
            (above as u8, below as u8, nan as i32, small as i32),
            "P1: float-to-integer casts. Left = your prediction, right = what Rust computes."
        );

        // P2. Remainder and division of a negative number, in two styles.
        let p2: (i32, i32, i32, i32) = (-1, 2, -2, -3);
        assert_eq!(
            p2,
            (
                -7 % 3,
                (-7_i32).rem_euclid(3),
                -7 / 3,
                (-7_i32).div_euclid(3)
            ),
            "P2: `%`, rem_euclid, `/`, div_euclid. Left = your prediction, right = what Rust computes."
        );

        // P3. The same two bytes, written and read in different byte orders.
        let p3: ([u8; 2], u16) = ([0x12, 0x34], 0x3412);
        assert_eq!(
            p3,
            (0x1234_u16.to_be_bytes(), u16::from_le_bytes([0x12, 0x34])),
            "P3: byte orders. Left = your prediction, right = what Rust computes."
        );

        // P4. Does each parse succeed?
        let p4: (bool, bool, bool, bool) = (true, false, false, true);
        assert_eq!(
            p4,
            (
                "+7".parse::<u8>().is_ok(),
                " 7".parse::<u8>().is_ok(),
                "256".parse::<u8>().is_ok(),
                "1e3".parse::<f64>().is_ok()
            ),
            "P4: which texts parse. Left = your prediction, right = what Rust computes."
        );
    }
}
