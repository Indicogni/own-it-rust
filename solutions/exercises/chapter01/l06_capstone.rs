//! LESSON 06 — Capstone: the parking meter
//! Run: cargo test --test chapter01 l06_
//! Read first: "Lesson 06" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `minutes` is a `u32`, a whole number that cannot be negative. What can
//!    go wrong in `minutes - 30`?
//!
//! IMPLEMENT the tasks below without help from the lesson text: this is the
//! whole loop on your own. Task C is already written and is wrong; let its
//! failing test lead you. Write one test of your own in `mod my_tests`.
//! EXPLAIN afterward: Describe the loop as you really did it for task B: what
//! you predicted, what the first report said, and what you changed. Say which
//! step you were tempted to skip and what you would have lost.
//!
//! COMPILER / TRANSFER LAB (optional): Write task B first WITHOUT an `if`, as
//! one line of arithmetic, and run `cargo test --test chapter01 l06_`. Read
//! the panic message of the failed test. Then run
//! `cargo test --release --test chapter01 l06_` and compare the two reports.

/// A. A stay of `hours` hours and `minutes` minutes: return its length in
/// minutes.
fn total_minutes(hours: u32, minutes: u32) -> u32 {
    hours * 60 + minutes
}

/// B. Return the parking fee in cents for a stay of `minutes` minutes.
/// The first 30 minutes are free. Every minute after that costs 2 cents.
fn fee_in_cents(minutes: u32) -> u32 {
    // The subtraction only runs when `minutes` is larger than 30, so it can
    // never go below zero.
    if minutes > 30 { (minutes - 30) * 2 } else { 0 }
}

/// C. Parking is allowed for at most 120 minutes. Return true when a stay of
/// `minutes` minutes is allowed.
fn may_park(minutes: u32) -> bool {
    // "At most 120" includes 120 itself.
    minutes <= 120
}

/// D. Your own test. Write one function here that starts with the line
/// `#[test]` and checks one more case of `total_minutes`, `fee_in_cents` or
/// `may_park`.
#[cfg(test)]
mod my_tests {
    use super::*;

    // Write your test below this line.

    #[test]
    fn a_whole_day_in_minutes() {
        // The supplied tests stop at two hours.
        assert_eq!(total_minutes(24, 0), 1440);
        assert!(!may_park(total_minutes(24, 0)));
    }
}

// ---- supplied tests: keep everything below this line as it is ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hours_and_minutes_become_minutes() {
        assert_eq!(total_minutes(1, 30), 90);
        assert_eq!(total_minutes(0, 45), 45);
        assert_eq!(total_minutes(2, 0), 120);
        assert_eq!(total_minutes(0, 0), 0);
    }

    #[test]
    fn b_fee_after_the_free_half_hour() {
        assert_eq!(fee_in_cents(31), 2);
        assert_eq!(fee_in_cents(45), 30);
        assert_eq!(fee_in_cents(120), 180);
    }

    #[test]
    fn b_the_first_half_hour_is_free() {
        assert_eq!(fee_in_cents(30), 0, "exactly 30 minutes are still free");
        assert_eq!(
            fee_in_cents(10),
            0,
            "a stay of 10 minutes is free. A `u32` cannot hold 10 - 30, so that \
             subtraction must not run for short stays"
        );
        assert_eq!(fee_in_cents(0), 0, "no stay, no fee");
    }

    #[test]
    fn c_two_hours_are_the_limit() {
        assert!(may_park(5), "5 minutes are allowed");
        assert!(may_park(119), "119 minutes are allowed");
        assert!(may_park(120), "exactly 120 minutes are still allowed");
        assert!(!may_park(121), "121 minutes are too long");
    }

    #[test]
    fn d_you_wrote_a_test_of_your_own() {
        // This test reads the text of this file and counts the `#[test]`
        // lines between `mod my_tests` and the supplied tests.
        let source = include_str!("l06_capstone.rs");
        let start = source.find("mod my_tests").expect("keep `mod my_tests`");
        let end = source
            .find("// ---- supplied tests")
            .expect("keep the line that starts with `// ---- supplied tests`");
        let own_tests = source[start..end]
            .lines()
            .filter(|line| line.trim() == "#[test]")
            .count();
        assert!(
            own_tests >= 1,
            "task D: `mod my_tests` has no test yet. Write a function there with the line \
             `#[test]` directly above it"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. A stay of 135 minutes: how many full hours, and how many
        // minutes are left over?
        let p1: (u32, u32) = (2, 15); // 135 = 2 * 60 + 15
        assert_eq!(
            p1,
            (135 / 60, 135 % 60),
            "P1: (135 / 60, 135 % 60) (left: your prediction, right: actual)"
        );

        // P2. What value does `fee` get?
        let minutes = 30;
        let fee = if minutes > 30 { minutes * 2 } else { 7 };
        let p2: u32 = 7; // 30 > 30 is false, so the else branch gives the value
        assert_eq!(
            p2, fee,
            "P2: the value of the if/else when minutes is exactly 30 \
             (left: your prediction, right: actual)"
        );
    }
}
