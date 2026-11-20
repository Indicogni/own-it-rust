//! LESSON 03 — if, loops and ranges
//! Run: cargo test --test chapter02 l03_
//! Read first: "Lesson 03" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. How many numbers does `0..5` produce? And `0..=5`?
//! 2. What does `continue` skip, and what does `break` end?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Say when you choose `for`, when `while` and when
//! `loop`, and what the difference between `a..b` and `a..=b` is.
//!
//! COMPILER / TRANSFER LAB (optional): In examples/ch02_scratch.rs, write
//! `let size = if 3 > 2 { "big" };` without an `else`. Read the error. Why
//! does an `if` that produces a value need an `else`? Repair it and leave the
//! file compiling.

/// A. Return "negative", "zero" or "positive" for `n`.
/// Use `if`, `else if` and `else` as ONE expression; do not write `return`.
fn sign_label(n: i32) -> &'static str {
    if n < 0 {
        "negative"
    } else if n == 0 {
        "zero"
    } else {
        "positive"
    }
}

/// B. Return the sum 1 + 2 + ... + n. For n = 0 the sum is 0.
/// Use a `for` loop over a range.
fn sum_up_to(n: u32) -> u32 {
    let mut sum = 0;
    for i in 1..=n {
        sum += i;
    }
    sum
}

/// C. Halve `n` again and again with whole-number division (`n / 2`) until it
/// is 0. Return how many halvings that took: 8 becomes 4, 2, 1, 0, so the
/// answer for 8 is 4. The answer for 0 is 0. Use a `while` loop.
fn halvings(n: u32) -> u32 {
    // A parameter is a variable of this function; shadowing it with a `mut`
    // variable lets the loop change it.
    let mut n = n;
    let mut count = 0;
    while n > 0 {
        n /= 2;
        count += 1;
    }
    count
}

/// D. Return the smallest multiple of `step` that is larger than `limit`.
/// For step 7 and limit 30 that is 35; for step 5 and limit 20 it is 25.
/// `step` is at least 1. Use `loop` and `break`.
fn first_multiple_above(step: u32, limit: u32) -> u32 {
    let mut multiple = step;
    loop {
        if multiple > limit {
            break;
        }
        multiple += step;
    }
    multiple
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::within;

    #[test]
    fn a_sign_label() {
        assert_eq!(sign_label(-5), "negative");
        assert_eq!(sign_label(0), "zero");
        assert_eq!(sign_label(12), "positive");
        assert_eq!(sign_label(i32::MIN), "negative");
    }

    #[test]
    fn b_sum_up_to() {
        assert_eq!(sum_up_to(4), 10);
        assert_eq!(sum_up_to(1), 1);
        assert_eq!(sum_up_to(0), 0, "the range 1..=0 is empty, so the sum is 0");
        assert_eq!(
            sum_up_to(100),
            5050,
            "if you got 4950, the range stopped one number early: use ..= to include n"
        );
    }

    // A loop that never ends would make the test wait forever. `within` runs
    // the call on a second thread and gives up after five seconds.
    const LOOP_HELP: &str =
        "(if this never ends: does the loop change its variable in every round?)";

    #[test]
    fn c_halvings() {
        for (n, expected) in [(8, 4), (1, 1), (0, 0), (9, 4), (1000, 10)] {
            let got = within(5, &format!("halvings({n}) {LOOP_HELP}"), move || {
                halvings(n)
            });
            assert_eq!(
                got, expected,
                "halvings({n}) (left: your result, right: expected)"
            );
        }
    }

    #[test]
    fn d_first_multiple_above() {
        for (step, limit, expected) in
            [(7, 30, 35), (5, 20, 25), (3, 0, 3), (1, 9, 10), (10, 3, 10)]
        {
            let got = within(
                5,
                &format!("first_multiple_above({step}, {limit}) {LOOP_HELP}"),
                move || first_multiple_above(step, limit),
            );
            assert_eq!(
                got, expected,
                "first_multiple_above({step}, {limit}): the result must be LARGER than the \
                 limit, not equal to it (left: your result, right: expected)"
            );
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. How many rounds does each loop make?
        let mut half_open = 0;
        for _ in 0..5 {
            half_open += 1;
        }
        let mut closed = 0;
        for _ in 0..=5 {
            closed += 1;
        }
        let p1: (i32, i32) = (5, 6); // 0..5 stops before 5; 0..=5 includes it
        assert_eq!(
            p1,
            (half_open, closed),
            "P1: rounds of `0..5` and of `0..=5` (left: your prediction, right: actual)"
        );

        // P2. `continue` and `break` in one loop. What is `total` at the end?
        let mut total = 0;
        for i in 1..10 {
            if i % 2 == 0 {
                continue;
            }
            if i > 6 {
                break;
            }
            total += i;
        }
        let p2: i32 = 9; // 1 + 3 + 5; 7 ends the loop
        assert_eq!(
            p2, total,
            "P2: total after the loop (left: your prediction, right: actual)"
        );

        // P3. `.rev()` walks a range backwards. Which number is built?
        let mut number = 0;
        for digit in (1..4).rev() {
            number = number * 10 + digit;
        }
        let p3: i32 = 321; // 1..4 is 1, 2, 3; reversed: 3, 2, 1
        assert_eq!(
            p3, number,
            "P3: the number built from (1..4).rev() (left: your prediction, right: actual)"
        );
    }
}
