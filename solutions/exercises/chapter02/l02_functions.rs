//! LESSON 02 — Functions, expressions and statements
//! Run: cargo test --test chapter02 l02_
//! Read first: "Lesson 02" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. What does a function return when its last line ends with a semicolon?
//! 2. Is `{ let a = 2; a * 10 }` a value that you can bind with `let`?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Explain the difference between an expression and a
//! statement, and what the final line of a function body without a semicolon
//! means.
//!
//! COMPILER / TRANSFER LAB (optional): In examples/ch02_scratch.rs, write a
//! function `fn double(n: i32) -> i32 { n * 2; }` and call it from `main`.
//! Read the error: which type did the compiler expect, which did it find, and
//! what does the help line propose? Repair it and leave the file compiling.

/// A. Return `n` multiplied by itself.
fn square(n: i64) -> i64 {
    n * n
}

/// B. Return the sum of the squares of `a` and `b`.
/// Call `square` twice; do not write the multiplications again.
fn sum_of_squares(a: i64, b: i64) -> i64 {
    square(a) + square(b)
}

/// C. Convert a temperature from degrees Celsius to degrees Fahrenheit:
/// multiply by 9, divide by 5, add 32. All numbers are `f64`, so write the
/// constants with a decimal point: `9.0`, not `9`.
fn to_fahrenheit(celsius: f64) -> f64 {
    celsius * 9.0 / 5.0 + 32.0
}

/// D. Return two values at once, as a tuple: the perimeter and the area of a
/// rectangle, in that order.
fn perimeter_and_area(width: u32, height: u32) -> (u32, u32) {
    let perimeter = 2 * (width + height);
    let area = width * height;
    (perimeter, area)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_square() {
        assert_eq!(square(3), 9);
        assert_eq!(square(-4), 16);
        assert_eq!(square(0), 0);
        assert_eq!(square(100_000), 10_000_000_000);
    }

    #[test]
    fn b_sum_of_squares() {
        assert_eq!(sum_of_squares(3, 4), 25);
        assert_eq!(sum_of_squares(-1, 1), 2);
        assert_eq!(sum_of_squares(0, 0), 0);
    }

    #[test]
    fn c_celsius_to_fahrenheit() {
        assert_eq!(to_fahrenheit(100.0), 212.0);
        assert_eq!(to_fahrenheit(0.0), 32.0);
        assert_eq!(to_fahrenheit(-40.0), -40.0);
        // 37 degrees Celsius is 98.6 Fahrenheit, but an f64 cannot store 98.6
        // exactly. Tests on floats allow a very small difference.
        let body = to_fahrenheit(37.0);
        assert!(
            (body - 98.6).abs() < 1e-9,
            "to_fahrenheit(37.0) should be 98.6 (within a tiny rounding error), got {body}"
        );
    }

    #[test]
    fn d_two_results_in_one_tuple() {
        assert_eq!(
            perimeter_and_area(3, 4),
            (14, 12),
            "the perimeter comes first, then the area"
        );
        assert_eq!(perimeter_and_area(1, 1), (4, 1));
        assert_eq!(perimeter_and_area(5, 0), (10, 0));
        // A tuple can be taken apart again with a pattern.
        let (perimeter, area) = perimeter_and_area(2, 10);
        assert_eq!(perimeter, 24);
        assert_eq!(area, 20);
    }

    // Two small functions for the predictions.
    fn add_one(n: i32) -> i32 {
        n + 1
    }
    fn double(n: i32) -> i32 {
        n * 2
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. A block is an expression. What value does `y` get?
        let y = {
            let a = 2;
            a * 10
        };
        let p1: i32 = 20;
        assert_eq!(
            p1, y,
            "P1: the value of the block (left: your prediction, right: actual)"
        );

        // P2. The inner call runs first. What are these two values?
        let p2: (i32, i32) = (7, 8); // double(3) + 1, and double(4)
        assert_eq!(
            p2,
            (add_one(double(3)), double(add_one(3))),
            "P2: (add_one(double(3)), double(add_one(3))) \
             (left: your prediction, right: actual)"
        );

        // P3. The block has its own `x`. What are `x` and `z` afterwards?
        let x = 5;
        let z = {
            let x = x + 1;
            x * 2
        };
        let p3: (i32, i32) = (5, 12); // the outer x is untouched
        assert_eq!(
            p3,
            (x, z),
            "P3: (x, z) after the block (left: your prediction, right: actual)"
        );
    }
}
