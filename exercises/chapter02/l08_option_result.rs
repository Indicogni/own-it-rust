//! LESSON 08 — Option and Result at first contact
//! Run: cargo test --test chapter02 l08_
//! Read first: "Lesson 08" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. What does `pop` return for a vector that is already empty?
//! 2. Is `"4 2".parse::<i32>()` an `Ok` or an `Err`?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Say when a function should return `Option` and when
//! `Result`, and why Rust has no "null" value that every variable may hold.
//!
//! COMPILER / TRANSFER LAB (optional): In examples/ch02_scratch.rs, write
//! `let n: i32 = "42".parse();`. Read the error: what does `parse` return?
//! Repair it once with `match` and once with `unwrap_or`. Leave the file
//! compiling.

/// A. Return the first even value in the list, or `None` when there is none.
fn first_even(values: &[i32]) -> Option<i32> {
    todo!("A: return Some(value) from inside the loop; None after it")
}

/// B. Describe an age that may be unknown: `Some(30)` gives "30 years",
/// `None` gives "unknown".
fn describe_age(age: Option<u32>) -> String {
    todo!("B: a match on `age` with an arm for Some and an arm for None")
}

/// C. Divide `a` by `b` with whole numbers. When `b` is 0, return
/// `Err` with the text "cannot divide by zero".
fn divide(a: i32, b: i32) -> Result<i32, String> {
    todo!("C: Err(String::from(...)) or Ok(...)")
}

/// D. Turn a text into a number. When the text is not a whole number,
/// return 0.
fn parse_or_zero(text: &str) -> i32 {
    todo!("D: text.parse::<i32>() gives a Result")
}

/// E. Both texts should be whole numbers. Return their sum. When one of
/// them is not a number, return `Err` with a message that names it: for
/// ("7", "x") return `Err` with the text "not a number: x". When both are
/// wrong, name the first one.
fn sum_of_two(a: &str, b: &str) -> Result<i32, String> {
    todo!("E: parse a, then parse b; an Err ends the function early")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;

    #[test]
    fn a_first_even() {
        assert_eq!(first_even(&[1, 3, 4, 6]), Some(4));
        assert_eq!(first_even(&[2]), Some(2));
        assert_eq!(first_even(&[1, 3, 5]), None);
        assert_eq!(first_even(&[]), None);
        assert_eq!(
            first_even(&[7, 0, 8]),
            Some(0),
            "0 is even, and it comes before 8"
        );
    }

    #[test]
    fn b_describe_age() {
        assert_eq!(describe_age(Some(30)), "30 years");
        assert_eq!(describe_age(Some(0)), "0 years");
        assert_eq!(describe_age(None), "unknown");
    }

    #[test]
    fn c_divide() {
        assert_eq!(divide(12, 4), Ok(3));
        assert_eq!(divide(7, 2), Ok(3));
        assert_eq!(divide(-9, 3), Ok(-3));
        assert_eq!(divide(5, 0), Err(String::from("cannot divide by zero")));
        assert_eq!(
            divide(0, 5),
            Ok(0),
            "0 divided by 5 is fine; only b must not be 0"
        );
    }

    #[test]
    fn d_parse_or_zero() {
        assert_eq!(parse_or_zero("42"), 42);
        assert_eq!(parse_or_zero("-7"), -7);
        assert_eq!(parse_or_zero("seven"), 0);
        assert_eq!(parse_or_zero(""), 0);
        assert_eq!(
            parse_or_zero("4 2"),
            0,
            "a text with a space inside is not a number"
        );
    }

    #[test]
    fn e_sum_of_two() {
        assert_eq!(sum_of_two("7", "5"), Ok(12));
        assert_eq!(sum_of_two("-1", "1"), Ok(0));
        assert_eq!(sum_of_two("7", "x"), Err(String::from("not a number: x")));
        assert_eq!(
            sum_of_two("seven", "5"),
            Err(String::from("not a number: seven"))
        );
        assert_eq!(
            sum_of_two("p", "q"),
            Err(String::from("not a number: p")),
            "when both texts are wrong, the message names the first one"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Two pops on a vector with one element.
        let mut v = vec![7];
        let first = v.pop();
        let second = v.pop();
        let p1: (Option<i32>, Option<i32>) = guess();
        assert_eq!(
            p1,
            (first, second),
            "P1: the results of two pops (left: your prediction, right: actual)"
        );

        // P2. Which of these four texts can `parse` turn into an i32?
        // Answer true or false for each, in square brackets and in the
        // order of the four texts: `[true, false, ...]`.
        let p2: [bool; 4] = guess();
        assert_eq!(
            p2,
            [
                "42".parse::<i32>().is_ok(),
                "4 2".parse::<i32>().is_ok(),
                "-7".parse::<i32>().is_ok(),
                "seven".parse::<i32>().is_ok(),
            ],
            "P2: is_ok() for \"42\", \"4 2\", \"-7\" and \"seven\" \
             (left: your prediction, right: actual)"
        );

        // P3. `unwrap_or` gives the value inside, or the spare value.
        // `present` is Some(3) and `absent` is None.
        let mut numbers = vec![3];
        let present = numbers.pop();
        let absent = numbers.pop();
        let p3: i32 = guess();
        assert_eq!(
            p3,
            present.unwrap_or(10) + absent.unwrap_or(10),
            "P3: Some(3).unwrap_or(10) + None.unwrap_or(10) \
             (left: your prediction, right: actual)"
        );
    }
}
