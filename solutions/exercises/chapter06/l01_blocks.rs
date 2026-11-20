//! LESSON 01 — Expression blocks and declarations
//! Run: cargo test --test chapter06 l01_
//! Read first: CONCEPTS.md, section "Lesson 01".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: What are the types of { 3 } and
//! { 3; }? Can a shadowed binding have a different type?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain tail expressions, unit, shadowing versus mutation,
//! where nested function items can be used, and when a temporary value created
//! in a block's tail expression is dropped.
//! COMPILER / TRANSFER LAB: Write an if whose branches have i32 and () types;
//! repair the semicolon. Declare a nested function after its call and compare its
//! visibility with a local let binding. Try reading a local variable from that
//! function item.
//! Use examples/ch06_scratch.rs; restore it to compiling code afterwards.

/// A. Use a block expression to trim text, then shadow the trimmed &str with
/// its byte length. Return (length, length_is_zero), without allocating.
fn inspected(text: &str) -> (usize, bool) {
    // A block is an expression. Inside it, `text` is shadowed by the trimmed
    // &str; the tail expression (no semicolon) is the block's value. The outer
    // `let` then shadows the &str parameter with a usize.
    let text = {
        let text = text.trim();
        text.len()
    };
    (text, text == 0)
}
/// B. Define a nested function item that doubles i16 into i32; use it to
/// return the sum of the doubled inputs. Put the item after its first call.
fn local_item(a: i16, b: i16) -> i32 {
    let sum = doubled(a) + doubled(b);
    // An item is visible in its whole block, even before its declaration.
    // Widen first: i16::MAX * 2 does not fit in an i16.
    fn doubled(n: i16) -> i32 {
        i32::from(n) * 2
    }
    sum
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::source;
    use practice::alloc::count;
    use std::cell::RefCell;
    #[test]
    fn a_tail_expression_returns_pair() {
        assert_eq!(inspected(" é "), (2, false));
        assert_eq!(inspected(" \n"), (0, true));
        assert_eq!(inspected("\u{3000}港\u{3000}"), (3, false));
        assert_eq!(inspected(""), (0, true));
    }
    #[test]
    fn b_item_scope_and_widening() {
        assert_eq!(local_item(4, -1), 6);
        assert_eq!(local_item(i16::MAX, i16::MAX), 131068);
        assert_eq!(local_item(i16::MIN, i16::MIN), -131072);
    }

    #[test]
    fn a_inspected_does_not_allocate() {
        let text = String::from("  padded label  ");
        let (result, allocations) = count(|| inspected(&text));
        assert_eq!(result, (12, false));
        assert_eq!(allocations, 0, "trim borrows; it never needs a new String");
    }

    #[test]
    fn b_source_declares_a_nested_fn() {
        let body = source::body(include_str!("l01_blocks.rs"), "local_item");
        assert!(
            source::has_word(&body, "fn"),
            "task B asks for a nested function item: declare a `fn` inside the body of \
             local_item (this test reads your source)"
        );
    }

    /// Logs its name when it is dropped.
    struct Noisy<'a>(&'static str, &'a RefCell<Vec<&'static str>>);
    impl Noisy<'_> {
        fn name(&self) -> &'static str {
            self.0
        }
    }
    impl Drop for Noisy<'_> {
        fn drop(&mut self) {
            self.1.borrow_mut().push(self.0);
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. A block is an expression; its value is its tail.
        let x = {
            let y = 3;
            y * 2
        } + 1;
        let p1: i32 = 7;
        assert_eq!(p1, x, "P1: value of a block used inside an expression");

        // P2. Size in bytes of each block's value: with a tail, and ending in `;`.
        let mut total = 0u64;
        let with_tail = {
            total += 5;
            total
        };
        #[allow(clippy::let_unit_value)]
        let with_semicolon = {
            total += 5;
        };
        assert_eq!(total, 10);
        let p2: (usize, usize) = (8, 0);
        assert_eq!(
            p2,
            (size_of_val(&with_tail), size_of_val(&with_semicolon)),
            "P2: a trailing semicolon discards the value"
        );

        // P3. Shadowing inside an inner block, then reading the outer binding.
        let n = 1;
        {
            let n = n + 10;
            assert_eq!(n, 11);
        }
        let n = n * 5;
        let p3: i32 = 5;
        assert_eq!(p3, n, "P3: an inner shadow ends with its block");

        // P4. `Noisy("tail", &log)` is a temporary value: it has no name, and it
        // is created inside the block's tail expression. In what order are the
        // three Noisy values dropped? (This course uses edition 2024.)
        let log = RefCell::new(Vec::new());
        let length = {
            let _first = Noisy("first", &log);
            let _second = Noisy("second", &log);
            Noisy("tail", &log).name().len()
        };
        assert_eq!(length, 4);
        let p4: Vec<&str> = vec!["tail", "second", "first"];
        assert_eq!(
            p4,
            *log.borrow(),
            "P4: a temporary in a tail expression is dropped before the block's locals"
        );
    }
}
