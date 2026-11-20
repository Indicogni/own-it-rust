//! LESSON 05 — The lesson loop: predict, implement, explain
//! Run: cargo test --test chapter01 l05_
//! Read first: "Lesson 05" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `10 * 15 / 100` and `10 / 100 * 15` use the same three numbers. Do they
//!    give the same result when all the numbers are whole numbers?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests, and write one test of
//! your own in `mod my_tests` (task D).
//! EXPLAIN afterward: Explain why the order of `*` and `/` matters in task C,
//! and what your own test checks that the supplied tests did not.
//!
//! COMPILER / TRANSFER LAB (optional): Run `cargo progress` before and after
//! this lesson and compare the line for chapter 1. Then open HINTS.md, find
//! this lesson, and read "Check your explanation" next to your own text.

/// A. Return the price of `items` things that cost `unit_price` cents each.
fn price_of(items: u32, unit_price: u32) -> u32 {
    items * unit_price
}

/// B. Return true when `n` is an even number: 0, 2, 4, ... and also -2,
/// -4, ...
fn is_even(n: i32) -> bool {
    // Compare with 0, not with 1: for negative odd numbers the remainder is -1.
    n % 2 == 0
}

/// C. Add a tip of `percent` percent to a bill of `bill` cents and return the
/// new total in whole cents. A part of a cent is dropped: a 10 percent tip on
/// 55 cents is 5 cents.
fn with_tip(bill: u32, percent: u32) -> u32 {
    // Multiply first and divide last. `bill / 100 * percent` would drop the
    // cents of the bill before the multiplication.
    bill + bill * percent / 100
}

/// D. Your own test. Write one function here that starts with the line
/// `#[test]` and checks one more case of `price_of`, `is_even` or `with_tip`:
/// a case that the supplied tests do not have. CONCEPTS.md, "Lesson 05",
/// shows the shape of a test.
#[cfg(test)]
mod my_tests {
    use super::*;

    // Write your test below this line.

    #[test]
    fn tip_on_one_cent_is_dropped() {
        // 10 percent of 1 cent is a tenth of a cent, and parts of a cent are
        // dropped. The supplied tests have no bill this small.
        assert_eq!(with_tip(1, 10), 1);
    }
}

// ---- supplied tests: keep everything below this line as it is ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_price_is_items_times_unit_price() {
        assert_eq!(price_of(3, 250), 750);
        assert_eq!(price_of(1, 99), 99);
        assert_eq!(price_of(0, 500), 0);
    }

    #[test]
    fn b_even_numbers() {
        assert!(is_even(0), "0 is even");
        assert!(is_even(2), "2 is even");
        assert!(is_even(1000), "1000 is even");
        assert!(!is_even(1), "1 is odd");
        assert!(!is_even(999), "999 is odd");
        assert!(is_even(-4), "-4 is even");
        assert!(
            !is_even(-3),
            "-3 is odd. The remainder of -3 divided by 2 is -1, not 1: \
             compare the remainder with 0"
        );
    }

    #[test]
    fn c_tip_is_added_in_whole_cents() {
        assert_eq!(with_tip(2000, 10), 2200);
        assert_eq!(with_tip(2000, 0), 2000);
        assert_eq!(with_tip(55, 10), 60);
    }

    #[test]
    fn c_small_bills_still_get_a_tip() {
        assert_eq!(
            with_tip(50, 10),
            55,
            "a 10 percent tip on 50 cents is 5 cents. If you got 50, you divided by 100 \
             before you multiplied, and whole-number division dropped everything: \
             multiply first, divide last"
        );
    }

    #[test]
    fn d_you_wrote_a_test_of_your_own() {
        // This test reads the text of this file and counts the `#[test]`
        // lines between `mod my_tests` and the supplied tests.
        let source = include_str!("l05_lesson_loop.rs");
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
        // P1. Whole-number division and remainder. What are these two values?
        let p1: (u32, u32) = (3, 1); // 7 = 3 * 2 + 1
        assert_eq!(
            p1,
            (7 / 2, 7 % 2),
            "P1: (7 / 2, 7 % 2) for whole numbers (left: your prediction, right: actual)"
        );

        // P2. The same three numbers in two orders.
        let bill = 10;
        let percent = 15;
        let p2: (u32, u32) = (1, 0); // 150 / 100 is 1, but 10 / 100 is already 0
        assert_eq!(
            p2,
            (bill * percent / 100, bill / 100 * percent),
            "P2: (10 * 15 / 100, 10 / 100 * 15) (left: your prediction, right: actual)"
        );

        // P3. `&&` means "and". Is this true or false?
        let p3: bool = false; // the second half, 2 > 5, is false
        assert_eq!(
            p3,
            3 > 2 && 2 > 5,
            "P3: 3 > 2 && 2 > 5 (left: your prediction, right: actual)"
        );
    }
}
