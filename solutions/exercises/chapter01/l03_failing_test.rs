//! LESSON 03 — Reading a failing test
//! Run: cargo test --test chapter01 l03_
//! Read first: "Lesson 03" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. In a failed `assert_eq!(a, b)`, which of the two is printed as `left`?
//! 2. A test calls a function, and the function itself panics. Which file and
//!    line does the report name: the test's or the function's?
//!
//! IMPLEMENT: this lesson is different. The four functions below are already
//! written, and each of them is WRONG. Do not read them for the mistake.
//! Run the lesson first, read what each failed test reports, and let the
//! report lead you to the line that you must change.
//! EXPLAIN afterward: Describe where in a failed test's report you find the
//! name of the test, the file and line, and the expected and the actual
//! value. Say how the report of task C differed from the report of task A.
//!
//! COMPILER / TRANSFER LAB (optional): Before you repair task C, run
//! `RUST_BACKTRACE=1 cargo test --test chapter01 l03_failing_test::tests::c_`
//! (in Windows PowerShell: `$env:RUST_BACKTRACE=1` on one line, then the
//! cargo command on the next). Find the line of the backtrace that names
//! `share_per_person`.

/// A. Return the number of seconds in `minutes` minutes.
fn seconds_in(minutes: u32) -> u32 {
    // The report said `left: 6, right: 60` for one minute: the factor was wrong.
    minutes * 60
}

/// B. A person may vote from the age of 18. Return true when a person of
/// `age` years may vote.
fn may_vote(age: u32) -> bool {
    // The report said "a person of exactly 18 may vote": 18 itself belongs
    // to the allowed ages, so the comparison needs `=`.
    age >= 18
}

/// C. Split `total` coins among `people` people and return how many whole
/// coins each person gets. When there are no people, nobody gets anything:
/// return 0.
fn share_per_person(total: u32, people: u32) -> u32 {
    // The report said "attempt to divide by zero" and named this function's
    // line. Dividing by zero must never run, so that case leaves the function
    // first. (`if people == 0 { 0 } else { total / people }` does the same.)
    if people == 0 {
        return 0;
    }
    total / people
}

/// D. Return the larger of the two numbers.
fn larger(a: i32, b: i32) -> i32 {
    // The two branches were the wrong way round.
    if a > b { a } else { b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seconds_in_minutes() {
        assert_eq!(seconds_in(1), 60);
        assert_eq!(seconds_in(5), 300);
        assert_eq!(seconds_in(0), 0);
    }

    #[test]
    fn b_voting_starts_at_18() {
        assert!(may_vote(40), "a person of 40 may vote");
        assert!(may_vote(19), "a person of 19 may vote");
        assert!(may_vote(18), "a person of exactly 18 may vote");
        assert!(!may_vote(17), "a person of 17 may not vote yet");
        assert!(!may_vote(0), "a baby may not vote");
    }

    #[test]
    fn c_coins_are_shared_in_whole_numbers() {
        assert_eq!(share_per_person(12, 4), 3);
        assert_eq!(share_per_person(7, 2), 3);
        assert_eq!(share_per_person(5, 0), 0);
    }

    #[test]
    fn d_larger_of_two() {
        assert_eq!(larger(2, 9), 9);
        assert_eq!(larger(9, 2), 9);
        assert_eq!(larger(-3, -8), -3);
        assert_eq!(larger(4, 4), 4);
    }

    /// Used by the predictions only.
    fn triple(n: i32) -> i32 {
        n + n + n
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Imagine a test with this line in it:
        //         assert_eq!(triple(4), 13);
        // It fails. Its report has a line `left: ...` and a line `right: ...`.
        // Which number stands after `left`, and which after `right`?
        // Answer as a pair in parentheses, in that order, such as `(1, 2)`.
        let p1: (i32, i32) = (12, 13); // left is the first argument, right the second
        assert_eq!(
            p1,
            (triple(4), 13),
            "P1: the (left, right) pair of the failed assertion \
             (left: your prediction, right: actual)"
        );

        // P2. How many of these four checks are true? (`!=` means "is not
        // equal to". The long line in the assertion below only counts the
        // true ones; you do not need to read it.)
        let checks = [
            triple(1) == 3,
            triple(3) > 9,
            triple(2) != 6,
            triple(0) == 0,
        ];
        let p2: usize = 2; // 3 == 3 and 0 == 0 hold; 9 > 9 and 6 != 6 do not
        assert_eq!(
            p2,
            checks.iter().filter(|check| **check).count(),
            "P2: how many of the four checks hold (left: your prediction, right: actual)"
        );
    }
}
