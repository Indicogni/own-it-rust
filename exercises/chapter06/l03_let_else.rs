//! LESSON 03 — if let and let else
//! Run: cargo test --test chapter06 l03_
//! Read first: CONCEPTS.md, section "Lesson 03".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Must let else have a diverging else
//! block? When is a variable bound by if let available afterward?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain refutable versus irrefutable patterns, the scope of
//! successful bindings, and how long a temporary value in the condition of an
//! if let lives compared with one in the scrutinee of a match.
//! COMPILER / TRANSFER LAB: Write let Some(x) = input else { 0 }; and inspect the
//! error. Repair using return. Compare an if let expression that provides a
//! fallback with a let else statement that guards the rest of a function.
//! Use examples/ch06_scratch.rs; restore it to compiling code afterwards.

/// A. Add a present i16 bonus to the i32 base using if let; absent means base.
fn bonus(base: i32, extra: Option<i16>) -> i32 {
    todo!("Implement this task")
}
/// B. Require a colon with let else: when `line` has no colon, the else block
/// returns None. The key is the text before the FIRST colon, trimmed. An empty
/// key is rejected with None too. Return the key borrowed from `line`.
///
/// Clippy would suggest the `?` operator for the colon guard. The allow below
/// keeps the let else that this lesson practises; Chapter 7 shows that `?` is
/// shorthand for exactly this pattern, so leave the attribute in place.
#[allow(clippy::question_mark)]
fn required_key(line: &str) -> Option<&str> {
    todo!("Implement this task")
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::source;
    use practice::guess;
    use std::cell::RefCell;
    #[test]
    fn a_if_let_has_an_absent_path() {
        assert_eq!(bonus(10, Some(-3)), 7);
        assert_eq!(bonus(10, None), 10);
        assert_eq!(bonus(0, Some(i16::MIN)), -32768);
        assert_eq!(bonus(i32::MAX - 1, Some(1)), i32::MAX);
    }
    #[test]
    fn b_let_else_guards_remainder() {
        assert_eq!(required_key(" mode :quiet"), Some("mode"));
        assert_eq!(required_key(":v"), None);
        assert_eq!(required_key("x"), None);
        assert_eq!(required_key("港:"), Some("港"));
        assert_eq!(required_key(" \t :v"), None);
        assert_eq!(required_key("a:b:c"), Some("a"));
        assert_eq!(required_key("\u{3000}key\u{3000}: v"), Some("key"));
    }
    #[test]
    fn b_key_is_borrowed_from_the_line() {
        let line = String::from("  name : value");
        let key = required_key(&line).unwrap();
        let start = line.as_ptr() as usize;
        assert_eq!(
            key.as_ptr() as usize,
            start + 2,
            "the key must be a slice of `line` that starts after the two leading spaces, \
             not a copy"
        );
    }

    #[test]
    fn a_source_uses_if_let() {
        let body = source::body(include_str!("l03_let_else.rs"), "bonus");
        assert!(
            source::has(&body, "if let "),
            "task A asks for if let inside bonus (this test reads your source)"
        );
    }

    #[test]
    fn b_source_uses_let_else() {
        let body = source::body(include_str!("l03_let_else.rs"), "required_key");
        assert!(
            source::has_let_else(&body),
            "task B asks for let else inside required_key: \
             `let PATTERN = VALUE else {{ return None; }};` (this test reads your source)"
        );
    }

    fn first_even(values: &[i32]) -> i32 {
        let [first, ..] = values else {
            return -1;
        };
        if first % 2 == 0 { *first } else { 0 }
    }

    /// Logs its name when it is dropped.
    struct Noisy<'a>(&'static str, &'a RefCell<Vec<&'static str>>);
    impl Noisy<'_> {
        /// Some(n) when n is even.
        fn even(&self, n: u32) -> Option<u32> {
            n.is_multiple_of(2).then_some(n)
        }
    }
    impl Drop for Noisy<'_> {
        fn drop(&mut self) {
            self.1.borrow_mut().push(self.0);
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. `while let` runs until the pattern stops matching.
        let mut stack = vec![1, 2, 3];
        let mut digits = 0;
        while let Some(top) = stack.pop() {
            digits = digits * 10 + top;
        }
        let p1: i32 = guess();
        assert_eq!(p1, digits, "P1: while let over pop");

        // P2. let else on a slice pattern: values for [], [3, 4] and [8].
        let p2: (i32, i32, i32) = guess();
        assert_eq!(
            p2,
            (first_even(&[]), first_even(&[3, 4]), first_even(&[8])),
            "P2: the else branch runs only when the pattern fails"
        );

        // P3. if let with a literal inside the pattern.
        let reading = Some(0u8.wrapping_sub(1));
        let p3: &str = guess();
        let kind = if let Some(255) = reading {
            "max"
        } else {
            "other"
        };
        assert_eq!(p3, kind, "P3: patterns can contain literals");

        // P4. `Noisy("temp", &log)` is a temporary value in the condition of an
        // if let. It logs "temp" when it is dropped. What does the log hold
        // after each call? (This course uses edition 2024.)
        fn with_if_let(n: u32, log: &RefCell<Vec<&'static str>>) -> u32 {
            if let Some(even) = Noisy("temp", log).even(n) {
                log.borrow_mut().push("then");
                even
            } else {
                log.borrow_mut().push("else");
                0
            }
        }
        let (matched, unmatched) = (RefCell::new(Vec::new()), RefCell::new(Vec::new()));
        assert_eq!(
            (with_if_let(4, &matched), with_if_let(3, &unmatched)),
            (4, 0)
        );
        let p4: (Vec<&str>, Vec<&str>) = guess();
        assert_eq!(
            p4,
            (matched.take(), unmatched.take()),
            "P4: the temporary lives through the `then` block but is dropped before `else`"
        );

        // P5. The same unmatched case written as a match.
        let log = RefCell::new(Vec::new());
        let value = match Noisy("temp", &log).even(3) {
            Some(even) => {
                log.borrow_mut().push("some");
                even
            }
            None => {
                log.borrow_mut().push("none");
                0
            }
        };
        assert_eq!(value, 0);
        let p5: Vec<&str> = guess();
        assert_eq!(
            p5,
            log.take(),
            "P5: a temporary in a match scrutinee lives until the end of the whole match"
        );
    }
}
