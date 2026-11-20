//! LESSON 03 — Generic enums
//! Run: cargo test --test chapter10 l03_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Can a generic enum carry unrelated types in different variants?
//! 2. Must either type implement Clone for a swap that consumes its input?
//!
//! IMPLEMENT the tasks below. Keep the tests and add one edge case.
//!
//! EXPLAIN afterward: Explain which type arguments change when variants are
//! exchanged and why matching can transfer ownership without bounds.
//!
//! COMPILER / TRANSFER LAB (optional, not checked by tests): Instantiate
//! Either with String/u32 and with two borrowed values. Add an unnecessary
//! Copy bound and explain which valid callers it excludes. Compare Either with
//! Result: what does each name tell the reader about the two sides?
//! Use examples/ch10_scratch.rs, and restore it to compiling code afterwards.

#[derive(Debug, PartialEq)]
enum Either<L, R> {
    Left(L),
    Right(R),
}
/// A. Consume and swap alternatives: Left(l) becomes Right(l), and vice versa.
fn swap<L, R>(value: Either<L, R>) -> Either<R, L> {
    todo!("Implement this task")
}
/// B. Borrow the payload while retaining its variant and without allocating.
fn borrowed<L, R>(value: &Either<L, R>) -> Either<&L, &R> {
    todo!("Implement this task")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, guess};
    #[test]
    fn a_swap_moves_noncopy_payloads() {
        assert_eq!(
            swap::<String, u32>(Either::Left("x".into())),
            Either::Right("x".into())
        );
        assert_eq!(swap::<String, u32>(Either::Right(9)), Either::Left(9));
        let s = String::from("moved");
        let p = s.as_ptr();
        match swap::<String, u8>(Either::Left(s)) {
            Either::Right(out) => assert_eq!(
                out.as_ptr(),
                p,
                "swap must move the String into the other variant, not copy it"
            ),
            Either::Left(_) => panic!("Left must become Right"),
        }
    }
    #[test]
    fn b_payload_borrow_keeps_identity() {
        let v: Either<String, u32> = Either::Left("x".into());
        match (borrowed(&v), &v) {
            (Either::Left(a), Either::Left(b)) => assert!(
                std::ptr::eq(a, b),
                "borrowed must point at the payload inside `value`, not at a copy"
            ),
            _ => panic!("wrong variant"),
        };
        assert_eq!(
            borrowed(&Either::<String, u32>::Right(4)),
            Either::Right(&4)
        );
        let (_, allocations) = count(|| borrowed(&v));
        assert_eq!(allocations, 0, "borrowing a payload never allocates");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Option derives Ord: write the sorted vector.
        let mut options = vec![Some(2), None, Some(1)];
        options.sort();
        let p1: Vec<Option<i32>> = guess();
        assert_eq!(p1, options, "P1: None versus Some");

        // P2. Result derives Ord too: write the sorted vector.
        let mut results: Vec<Result<i32, i32>> = vec![Err(1), Ok(9), Ok(3)];
        results.sort();
        let p2: Vec<Result<i32, i32>> = guess();
        assert_eq!(p2, results, "P2: Ok versus Err");

        // P3. map keeps the Ok variant and changes only its type argument.
        let r: Result<u8, ()> = Ok(250);
        let p3: Result<Option<u8>, ()> = guess();
        assert_eq!(p3, r.map(|n| n.checked_add(10)), "P3: nested generic enums");
    }
}
