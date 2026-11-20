//! LESSON 01 — Variables, mutability and basic types
//! Run: cargo test --test chapter02 l01_
//! Read first: "Lesson 01" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. After `let x = 5;`, is `x = 6;` allowed? Is `let x = 6;` allowed?
//! 2. What is `7 / 2` for whole numbers, and what is `7.0 / 2.0`?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Explain the difference between changing a `mut`
//! variable and shadowing a name with a second `let`, and say why Rust makes
//! you write `mut`.
//!
//! COMPILER / TRANSFER LAB (optional): In examples/ch02_scratch.rs, write
//! `let total = 1;` and on the next line `total = 2;`. Read the error and
//! what its help proposes. Repair it once with `mut` and once by shadowing.
//! Leave the file compiling afterward.

/// A. A shop adds a fee of 10 cents for every full 100 cents of the price.
/// Return the price plus the fee, in cents: 250 costs 270, and 99 costs 99.
/// Bind the fee to a variable with `let` first, then return the sum.
fn price_with_fee(price: u32) -> u32 {
    todo!("A: let fee = ...; then price + fee")
}

/// B. Start from `balance` and add `deposit` to it three times.
/// Use one `let mut` variable and the `+=` operator.
fn after_three_deposits(balance: i64, deposit: i64) -> i64 {
    todo!("B: let mut total = balance; then three times `total += deposit;`")
}

/// C. Return the average of two whole numbers as an `f64`.
/// The average of 1 and 2 is 1.5, not 1. Convert each number to `f64` first;
/// then add and divide.
fn average(a: i32, b: i32) -> f64 {
    todo!("C: f64::from(a) converts an i32 to an f64")
}

/// D. Return true when `c` is one of the capital letters `A` to `Z`.
fn is_capital(c: char) -> bool {
    todo!("D: compare with the chars 'A' and 'Z', or find a method of char")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;

    #[test]
    fn a_fee_is_ten_cents_per_full_hundred() {
        assert_eq!(price_with_fee(250), 270);
        assert_eq!(price_with_fee(100), 110);
        assert_eq!(
            price_with_fee(99),
            99,
            "99 cents has no full 100 in it, so the fee is 0"
        );
        assert_eq!(price_with_fee(0), 0);
    }

    #[test]
    fn b_three_deposits_are_added() {
        assert_eq!(after_three_deposits(100, 25), 175);
        assert_eq!(after_three_deposits(0, 0), 0);
        assert_eq!(
            after_three_deposits(10, -20),
            -50,
            "an i64 can be negative: 10 - 20 - 20 - 20"
        );
    }

    #[test]
    fn c_average_keeps_the_half() {
        assert_eq!(
            average(1, 2),
            1.5,
            "if you got 1.0, the division was done with whole numbers: \
             convert to f64 first, then divide by 2.0"
        );
        assert_eq!(average(4, 6), 5.0);
        assert_eq!(average(-3, 0), -1.5);
        assert_eq!(
            average(i32::MAX, i32::MAX),
            f64::from(i32::MAX),
            "adding two large i32 values overflows: convert each to f64 before you add"
        );
    }

    #[test]
    fn d_capital_letters() {
        assert!(is_capital('A'), "'A' is the first capital letter");
        assert!(is_capital('Q'));
        assert!(is_capital('Z'), "'Z' is the last capital letter");
        assert!(!is_capital('a'), "'a' is a small letter");
        assert!(!is_capital('7'));
        assert!(
            !is_capital('['),
            "'[' comes directly after 'Z' but is not a letter"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Three `let x`. Which value does the LAST line see?
        let x = 5;
        let x = x * 2;
        {
            let x = x + 100;
            assert!(x > 100);
        }
        let p1: i32 = guess();
        assert_eq!(
            p1, x,
            "P1: x after the inner block has ended (left: your prediction, right: actual)"
        );

        // P2. The same division with whole numbers and with floats.
        let p2: (i32, f64) = guess();
        assert_eq!(
            p2,
            (7 / 2, 7.0 / 2.0),
            "P2: (7 / 2, 7.0 / 2.0) (left: your prediction, right: actual)"
        );

        // P3. One `mut` variable, changed three times.
        let mut n = 1;
        n += 4;
        n *= 2;
        n -= 3;
        let p3: i32 = guess();
        assert_eq!(
            p3, n,
            "P3: n after += 4, *= 2, -= 3 (left: your prediction, right: actual)"
        );
    }
}
