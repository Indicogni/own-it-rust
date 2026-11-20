//! LESSON 07 — Display, fmt::Write, and format_args!
//! Run: cargo test --test chapter17 l07_
//! Read first: "Lesson 07" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Can Display::fmt allocate freely? Must it propagate formatter
//! failures? Why does write! work with both String and I/O writers under different traits?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why is fmt::Error for formatting machinery failures rather than input
//! validation? How does format_args! pass formatting without an intermediate String?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch17_scratch.rs):
//! Implement a wrapper for fmt::Write that always returns Err(fmt::Error), then call your
//! write_title function and verify propagation. Then write a function with the return
//! type `fmt::Arguments<'static>` that builds a String `name` and returns
//! `format_args!("<{name}>")`. The compiler rejects it (E0515), because the arguments
//! only borrow the String. Repair it by passing a writer into the function. Note that
//! a plain `let args = format_args!("{}", String::from("temp"));` does compile on
//! this toolchain: the temporary lives as long as `args`.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete when its tests, predictions, and explanation are done. The lab is optional.

use std::fmt::{self, Write};
struct Progress {
    done: u32,
    total: u32,
}
impl fmt::Display for Progress {
    /// A. Render exactly "DONE/TOTAL"; alternate format renders "DONE of TOTAL".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{} of {}", self.done, self.total)
        } else {
            write!(f, "{}/{}", self.done, self.total)
        }
    }
}
/// B. Write "[" + supplied formatting arguments + "]" into any fmt::Write.
/// Propagate failure: stop at the first write that fails and return its error,
/// whether it is the '[', a part of the arguments, or the ']'.
/// Do not allocate an intermediate String.
fn write_title<W: Write + ?Sized>(out: &mut W, args: fmt::Arguments<'_>) -> fmt::Result {
    // Each `?` stops at the first failed write. `write_fmt` sends the pieces of
    // `args` straight to `out`; no String is built in between.
    out.write_char('[')?;
    out.write_fmt(args)?;
    out.write_char(']')
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_display_supports_its_documented_alternate_form() {
        let p = Progress { done: 3, total: 8 };
        assert_eq!(format!("{p}"), "3/8");
        assert_eq!(format!("{p:#}"), "3 of 8");
        assert_eq!(p.to_string(), "3/8");
        let none = Progress { done: 0, total: 0 };
        assert_eq!(format!("{none}|{none:#}"), "0/0|0 of 0");
    }
    #[test]
    fn b_format_args_forwarding_uses_the_target_writer() {
        let mut text = String::from("prefix");
        write_title(&mut text, format_args!("{}={:.1}", "temp", 3.5)).unwrap();
        assert_eq!(text, "prefix[temp=3.5]");
    }
    #[test]
    fn b_write_errors_propagate() {
        struct Fail;
        impl Write for Fail {
            fn write_str(&mut self, _: &str) -> fmt::Result {
                Err(fmt::Error)
            }
        }
        assert!(
            write_title(&mut Fail, format_args!("x")).is_err(),
            "the writer refused the output, so write_title must return Err"
        );
    }

    /// Refuses every fragment that contains `poison` and records the others.
    struct Refuse {
        poison: char,
        seen: String,
    }
    impl Refuse {
        fn new(poison: char) -> Self {
            Refuse {
                poison,
                seen: String::new(),
            }
        }
    }
    impl Write for Refuse {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            if s.contains(self.poison) {
                return Err(fmt::Error);
            }
            self.seen.push_str(s);
            Ok(())
        }
    }

    #[test]
    fn b_write_errors_propagate_from_every_part() {
        let mut out = Refuse::new('[');
        assert!(
            write_title(&mut out, format_args!("{}", "ok")).is_err(),
            "the opening bracket failed, so write_title must return Err"
        );
        assert_eq!(
            out.seen, "",
            "nothing may be written after the first failed write"
        );

        let mut out = Refuse::new('k');
        assert!(
            write_title(&mut out, format_args!("{}", "ok")).is_err(),
            "a write failed inside the forwarded arguments; do not discard that error"
        );
        assert_eq!(
            out.seen, "[",
            "nothing may be written after the first failed write"
        );

        let mut out = Refuse::new(']');
        assert!(
            write_title(&mut out, format_args!("{}", "ok")).is_err(),
            "the closing bracket failed, so write_title must return Err"
        );
        assert_eq!(out.seen, "[ok");

        let mut out = Refuse::new('!');
        assert!(write_title(&mut out, format_args!("{}", "ok")).is_ok());
        assert_eq!(out.seen, "[ok]");
    }

    /// Counts bytes and never allocates.
    struct Tally(usize);
    impl Write for Tally {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.0 += s.len();
            Ok(())
        }
    }

    #[test]
    fn b_write_title_does_not_build_a_string() {
        let mut out = Tally(0);
        let (result, allocations) =
            count(|| write_title(&mut out, format_args!("{}-{:>4}", "é", 7)));
        assert!(result.is_ok());
        assert_eq!(out.0, "[é-   7]".len());
        assert_eq!(
            allocations, 0,
            "write_title made {allocations} heap allocation(s): forward the arguments \
             to the writer; don't format! them into a String first"
        );
    }

    struct Id(u32);
    impl fmt::Display for Id {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "#{}", self.0)
        }
    }
    struct Padded;
    impl fmt::Display for Padded {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.pad("ok")
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. `Id` writes with write!. Is the requested width applied?
        let p1: String = "[#7]".to_string();
        assert_eq!(
            p1,
            format!("[{:>5}]", Id(7)),
            "P1: write! and the caller's width"
        );

        // P2. `Padded` uses Formatter::pad instead.
        let p2: String = "[  ok]".to_string();
        assert_eq!(
            p2,
            format!("[{:>4}]", Padded),
            "P2: Formatter::pad and the caller's width"
        );

        // P3. How many chars long is the Debug form of "a\tb"?
        let p3: usize = 6;
        assert_eq!(
            p3,
            format!("{:?}", "a\tb").chars().count(),
            "P3: Debug quotes and escapes"
        );
    }
}
