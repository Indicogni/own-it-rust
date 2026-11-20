//! LESSON 05 — Vectors: a list that can grow
//! Run: cargo test --test chapter02 l05_
//! Read first: "Lesson 05" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A vector has 3 elements. Which index does the last one have?
//! 2. What happens at run time when you read index 3 of that vector?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Explain what `for x in &values` gives you in each
//! round, and why a function that only reads a list takes `&[i32]`.
//!
//! COMPILER / TRANSFER LAB (optional): In examples/ch02_scratch.rs, write
//! `let numbers = Vec::new();` and on the next line `numbers.push(1);`. Read
//! the error and the help. Repair it and leave the file compiling.

/// A. Return the squares of 1, 2, ..., n in a vector: for 4 return
/// [1, 4, 9, 16]. For 0 return an empty vector.
fn squares(n: u32) -> Vec<u32> {
    todo!("A: Vec::new(), a for loop, push")
}

/// B. Return the sum of all values. The sum of an empty list is 0.
fn total(values: &[i32]) -> i32 {
    todo!("B: for value in values")
}

/// C. Return how many values are larger than `limit`.
fn count_above(values: &[i32], limit: i32) -> usize {
    todo!("C: count in a `let mut` variable")
}

/// D. Return a new vector with the even values only, in their old order.
fn keep_even(values: &[i32]) -> Vec<i32> {
    todo!("D: push the values that pass the check")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;

    #[test]
    fn a_squares() {
        assert_eq!(squares(4), [1, 4, 9, 16]);
        assert_eq!(squares(1), [1]);
        assert!(squares(0).is_empty(), "squares(0) has no elements");
        assert_eq!(
            squares(10).len(),
            10,
            "squares(n) has exactly n elements: 1 squared up to n squared"
        );
    }

    #[test]
    fn b_total() {
        assert_eq!(total(&[1, 2, 3]), 6);
        assert_eq!(total(&[-5, 5]), 0);
        assert_eq!(total(&[]), 0);
        // A whole vector can be lent to a `&[i32]` parameter with `&`.
        let readings = vec![10, 20, 30, 40];
        assert_eq!(total(&readings), 100);
        assert_eq!(readings.len(), 4, "the caller still has its vector");
    }

    #[test]
    fn c_count_above() {
        assert_eq!(count_above(&[1, 5, 10], 4), 2);
        assert_eq!(
            count_above(&[4, 4, 4], 4),
            0,
            "4 is not larger than 4: equal values do not count"
        );
        assert_eq!(count_above(&[], 0), 0);
        assert_eq!(count_above(&[-1, -2], -3), 2);
    }

    #[test]
    fn d_keep_even() {
        assert_eq!(keep_even(&[1, 2, 3, 4, 6]), [2, 4, 6]);
        assert_eq!(keep_even(&[0, -2, -3]), [0, -2]);
        assert!(keep_even(&[1, 3, 5]).is_empty());
        assert!(keep_even(&[]).is_empty());
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. push, pop, push. How long is `v`, and what is at index 3?
        let mut v = vec![1, 2, 3];
        v.push(4);
        v.pop();
        v.push(5);
        let p1: (usize, i32) = guess();
        assert_eq!(
            p1,
            (v.len(), v[3]),
            "P1: (v.len(), v[3]) (left: your prediction, right: actual)"
        );

        // P2. `vec![7; 3]` is not the same as `vec![7, 3]`.
        let mut w = vec![7; 3];
        w.push(1);
        let p2: (usize, i32) = guess();
        assert_eq!(
            p2,
            (w.len(), total(&w)),
            "P2: the length of vec![7; 3] after one push, and the sum of its elements \
             (left: your prediction, right: actual)"
        );

        // P3. A loop inside a loop. How many pushes happen?
        let mut out = Vec::new();
        for i in 0..4 {
            for j in 0..i {
                out.push(j);
            }
        }
        let p3: usize = guess();
        assert_eq!(
            p3,
            out.len(),
            "P3: out.len() after the nested loops (left: your prediction, right: actual)"
        );
    }
}
