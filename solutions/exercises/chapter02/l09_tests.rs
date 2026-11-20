//! LESSON 09 — Writing your own tests
//! Run: cargo test --test chapter02 l09_
//! Read first: "Lesson 09" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A test function has two `assert!` lines and the first one fails. Does
//!    the second one run?
//! 2. Which years would you try to catch a wrong leap-year function?
//!
//! IMPLEMENT the tasks below. In this lesson YOU write the tests: the two
//! functions under test are supplied and correct.
//! EXPLAIN afterward: Explain how you chose your test cases, and why four
//! tests of ordinary years would not have been enough.
//!
//! COMPILER / TRANSFER LAB (optional): Change one of your tests so that it
//! is wrong on purpose (expect 1900 to be a leap year) and run the lesson.
//! Read the report: the name is one that YOU chose. Is it clear enough to
//! tell you what broke without opening the file? Put the test right again.

/// Supplied and correct: you test it, you do not change it.
///
/// A year is a leap year when it can be divided by 4. Years that can be
/// divided by 100 are an exception: they are not leap years. Years that can
/// be divided by 400 are an exception to the exception: they are leap years.
/// So 2024 and 2000 are leap years; 2023 and 1900 are not.
fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Supplied and correct: you test it, you do not change it.
///
/// Returns how many percent `part` is of `whole`, in whole numbers:
/// `percent(1, 4)` is 25. Panics when `whole` is 0, because no answer makes
/// sense then.
fn percent(part: u32, whole: u32) -> u32 {
    assert!(whole != 0, "percent: `whole` must not be 0");
    part * 100 / whole
}

/// C. A test that can be pointed at ANY leap-year function.
///
/// `candidate` is a function that takes a year and returns a bool; call it
/// like any function: `candidate(2024)`. Check it with `assert!` or
/// `assert_eq!` on enough years that a correct function passes and a wrong
/// one makes an assertion fail. The supplied tests call `check_leap_year`
/// once with the correct function and once each with four wrong ones.
fn check_leap_year(candidate: fn(u32) -> bool) {
    // One year for each rule, and one ordinary year that is no leap year.
    assert!(candidate(2024), "2024 can be divided by 4");
    assert!(!candidate(2023), "2023 cannot be divided by 4");
    assert!(
        !candidate(1900),
        "1900 can be divided by 100 but not by 400"
    );
    assert!(candidate(2000), "2000 can be divided by 400");
}

/// A. Write at least three tests for `is_leap_year` in this module. Each
/// test is a function with the line `#[test]` above it and a name that says
/// what it checks.
///
/// B. Write one more test for `percent` that expects a panic: put
/// `#[should_panic]` between `#[test]` and the function, and call `percent`
/// with a `whole` of 0 inside.
#[cfg(test)]
mod my_tests {
    use super::*;

    // Write your tests below this line.

    #[test]
    fn a_year_divisible_by_4_is_a_leap_year() {
        assert!(is_leap_year(2024));
        assert!(is_leap_year(1996));
    }

    #[test]
    fn an_ordinary_year_is_not_a_leap_year() {
        assert!(!is_leap_year(2023));
        assert!(!is_leap_year(2026));
    }

    #[test]
    fn a_century_is_not_a_leap_year() {
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2100));
    }

    #[test]
    fn every_fourth_century_is_a_leap_year() {
        assert!(is_leap_year(2000));
        assert!(is_leap_year(1600));
    }

    #[test]
    fn percent_of_a_part() {
        assert_eq!(percent(1, 4), 25);
        assert_eq!(percent(0, 9), 0);
        assert_eq!(percent(9, 9), 100);
    }

    #[test]
    #[should_panic(expected = "must not be 0")]
    fn percent_of_nothing_panics() {
        percent(1, 0);
    }
}

// ---- supplied tests: keep everything below this line as it is ----

#[cfg(test)]
mod tests {
    use super::*;

    /// The part of this file between `mod my_tests` and the supplied tests.
    fn my_tests_source() -> &'static str {
        let source = include_str!("l09_tests.rs");
        let start = source.find("mod my_tests").expect("keep `mod my_tests`");
        let end = source
            .find("// ---- supplied tests")
            .expect("keep the line that starts with `// ---- supplied tests`");
        &source[start..end]
    }

    #[test]
    fn a_you_wrote_three_tests() {
        let all_tests = my_tests_source()
            .lines()
            .filter(|line| line.trim() == "#[test]")
            .count();
        // A test with `#[should_panic]` belongs to task B and is not counted.
        let panic_tests = my_tests_source()
            .lines()
            .filter(|line| line.trim().starts_with("#[should_panic"))
            .count();
        let own_tests = all_tests.saturating_sub(panic_tests);
        assert!(
            own_tests >= 3,
            "task A: `mod my_tests` has {own_tests} ordinary test(s) so far (a test with \
             `#[should_panic]` belongs to task B and is not counted); write at least \
             three functions there, each with the line `#[test]` on a line of its own \
             directly above it"
        );
    }

    #[test]
    fn b_you_wrote_a_should_panic_test() {
        let has_one = my_tests_source()
            .lines()
            .any(|line| line.trim().starts_with("#[should_panic"));
        assert!(
            has_one,
            "task B: `mod my_tests` has no test with `#[should_panic]` yet"
        );
    }

    // Four wrong leap-year functions. Each one is a mistake that people
    // really make.
    fn always_leap(_year: u32) -> bool {
        true
    }
    fn never_leap(_year: u32) -> bool {
        false
    }
    fn every_fourth_year(year: u32) -> bool {
        year.is_multiple_of(4)
    }
    fn forgets_the_400_rule(year: u32) -> bool {
        year.is_multiple_of(4) && !year.is_multiple_of(100)
    }

    /// True when `check_leap_year` accepts the correct function and stops
    /// with a failed assertion for `candidate`.
    fn rejects(candidate: fn(u32) -> bool) -> bool {
        // If this line panics, your checks reject the CORRECT function.
        check_leap_year(is_leap_year);
        // `catch_unwind` runs the closure and reports whether it panicked.
        std::panic::catch_unwind(|| check_leap_year(candidate)).is_err()
    }

    #[test]
    fn c_checks_accept_the_correct_function() {
        check_leap_year(is_leap_year);
    }

    #[test]
    fn c_checks_catch_a_function_that_always_says_yes() {
        assert!(
            rejects(always_leap),
            "your checks pass a function that calls EVERY year a leap year: \
             assert that some ordinary year, such as 2023, is not one"
        );
    }

    #[test]
    fn c_checks_catch_a_function_that_always_says_no() {
        assert!(
            rejects(never_leap),
            "your checks pass a function that calls NO year a leap year: \
             assert that some year, such as 2024, is one"
        );
    }

    #[test]
    fn c_checks_catch_the_every_fourth_year_mistake() {
        assert!(
            rejects(every_fourth_year),
            "your checks pass a function that only divides by 4. \
             Which year can be divided by 4 and is still not a leap year?"
        );
    }

    #[test]
    fn c_checks_catch_the_forgotten_400_rule() {
        assert!(
            rejects(forgets_the_400_rule),
            "your checks pass a function that calls no full century a leap year. \
             Which century year IS a leap year?"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Tests often compare numbers. Floats need care. Which of these
        // two comparisons are true? (`0.3_f64` is the number 0.3 with its
        // type written after it.)
        let sum = 0.1 + 0.2;
        let p1: (bool, bool) = (false, true); // 0.1 + 0.2 is 0.30000000000000004
        assert_eq!(
            p1,
            (sum == 0.3, (sum - 0.3_f64).abs() < 1e-9),
            "P1: (0.1 + 0.2 == 0.3, the difference is smaller than 0.000000001) \
             (left: your prediction, right: actual)"
        );

        // P2. What does `percent` return here? Remember whole-number division.
        let p2: (u32, u32) = (33, 66); // 100 / 3 and 200 / 3, rest dropped
        assert_eq!(
            p2,
            (percent(1, 3), percent(2, 3)),
            "P2: (percent(1, 3), percent(2, 3)) (left: your prediction, right: actual)"
        );
    }
}
