//! LESSON 04 — Literal, tuple and struct patterns
//! Run: cargo test --test chapter10 l04_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Does a plain name such as `x` in a pattern compare against an existing
//!    variable `x`, or create a new binding? What if `x` names a constant?
//! 2. When should you write `_` and when `_name`?
//!
//! IMPLEMENT the tasks below. Keep the tests and add one edge case.
//!
//! EXPLAIN afterward: Explain shadowing in patterns, ignored fields, and named
//! versus positional destructuring.
//!
//! COMPILER / TRANSFER LAB (optional, not checked by tests): Put a plain
//! variable name as the first match arm and read the warning about the later
//! arms. Compare `_` with `_held` when matching a String and then using the
//! source again. Rewrite one nested tuple pattern with named fields.
//! Use examples/ch10_scratch.rs, and restore it to compiling code afterwards.

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}
/// A. Use struct patterns: origin => "origin", x=0 => "vertical",
/// y=0 => "horizontal", otherwise => "plane". Order matters.
fn location(point: Point) -> &'static str {
    match point {
        Point { x: 0, y: 0 } => "origin",
        Point { x: 0, .. } => "vertical",
        Point { y: 0, .. } => "horizontal",
        Point { .. } => "plane",
    }
}
/// B. Destructure the tuple parameter in the signature. Sum only first and
/// third as i64; ignore the owned String without cloning it.
fn endpoint_sum((first, _, third): (i32, String, i32)) -> i64 {
    i64::from(first) + i64::from(third)
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::cell::RefCell;
    #[test]
    fn a_specific_patterns_precede_general_ones() {
        assert_eq!(location(Point { x: 0, y: 0 }), "origin");
        assert_eq!(location(Point { x: 0, y: 2 }), "vertical");
        assert_eq!(location(Point { x: 3, y: 0 }), "horizontal");
        assert_eq!(location(Point { x: -1, y: 2 }), "plane");
        assert_eq!(location(Point { x: i32::MIN, y: 0 }), "horizontal");
    }
    #[test]
    fn b_tuple_pattern_ignores_the_middle() {
        assert_eq!(endpoint_sum((3, "unused".into(), -1)), 2);
        assert_eq!(
            endpoint_sum((i32::MAX, String::new(), i32::MAX)),
            4294967294
        );
        assert_eq!(endpoint_sum((i32::MIN, "🦀".into(), i32::MIN)), -4294967296);
    }
    #[test]
    fn b_ignored_string_is_not_cloned() {
        let input = (1, String::from("payload"), 2);
        let (sum, allocations) = count(|| endpoint_sum(input));
        assert_eq!(sum, 3);
        assert_eq!(allocations, 0, "ignore the String; do not copy it");
    }

    struct Noisy<'a>(&'static str, &'a RefCell<Vec<&'static str>>);
    impl Drop for Noisy<'_> {
        fn drop(&mut self) {
            self.1.borrow_mut().push(self.0);
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. `_` does not bind; `_held` binds and moves. Where does "payload"
        // appear in each log?
        let log = RefCell::new(Vec::new());
        {
            let value = Some(Noisy("payload", &log));
            match value {
                Some(_) => log.borrow_mut().push("arm"),
                None => log.borrow_mut().push("none"),
            }
            log.borrow_mut().push("after match");
        }
        let p1a: Vec<&str> = vec!["arm", "after match", "payload"];
        assert_eq!(p1a, *log.borrow(), "P1a: matching with _");
        let log = RefCell::new(Vec::new());
        {
            let value = Some(Noisy("payload", &log));
            match value {
                Some(_held) => log.borrow_mut().push("arm"),
                None => log.borrow_mut().push("none"),
            }
            log.borrow_mut().push("after match");
        }
        let p1b: Vec<&str> = vec!["arm", "payload", "after match"];
        assert_eq!(p1b, *log.borrow(), "P1b: matching with _held");

        // P2. `x` is a local variable here, not a constant. Does the `x` in
        // the pattern compare with it or bind a new `x`?
        let x = 5;
        let y = match (1, 2) {
            (x, 2) => x * 10,
            _ => 0,
        };
        let p2: (i32, i32) = (10, 5);
        assert_eq!(
            p2,
            (y, x),
            "P2: a name that is not a constant binds; it does not compare"
        );

        // P3. `field: name` binds the field to a new name.
        let Point { y: x, .. } = Point { x: 1, y: 2 };
        let p3: i32 = 2;
        assert_eq!(p3, x, "P3: renaming in struct patterns");
    }
}
