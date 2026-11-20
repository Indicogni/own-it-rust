//! LESSON 04 — Reading a compiler error
//! Run: cargo test --test chapter01 l04_
//! Read first: "Lesson 04" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Which part of a compiler message tells you WHERE the problem is?
//! 2. When the compiler reports three errors, which one do you repair first?
//!
//! IMPLEMENT the tasks below. Each task shows a real message from the Rust
//! compiler and asks for one fact that you can read from it. The programs
//! that produced the messages are not shown: the message is enough.
//! EXPLAIN afterward: Name the parts of a compiler message in the order in
//! which you read them, and say why you repair the first error first.
//!
//! COMPILER / TRANSFER LAB (optional): Run `rustc --explain E0425` and read
//! the first screen. Then run `cargo drill ch01`: ten small programs, most of
//! which do not compile. This is where you practise this lesson for real.

/// The compiler printed this for a small program in `src/main.rs`.
const FIRST_MESSAGE: &str = r#"
error[E0425]: cannot find value `totl` in this scope
 --> src/main.rs:3:34
  |
3 |     println!("The answer is {}", totl);
  |                                  ^^^^
  |
help: a local variable with a similar name exists
  |
3 |     println!("The answer is {}", total);
  |                                     +

For more information about this error, try `rustc --explain E0425`.
error: could not compile `hello` (bin "hello") due to 1 previous error
"#;

/// The compiler printed this for another small program.
const SECOND_MESSAGE: &str = r#"
error[E0308]: mismatched types
 --> src/main.rs:2:22
  |
2 |     let seats: u32 = "four";
  |                ---   ^^^^^^ expected `u32`, found `&str`
  |                |
  |                expected due to this

For more information about this error, try `rustc --explain E0308`.
error: could not compile `hello` (bin "hello") due to 1 previous error
"#;

/// The compiler printed all of this for ONE program with several mistakes.
const LONG_OUTPUT: &str = r#"
error[E0425]: cannot find value `cuont` in this scope
 --> src/main.rs:4:26
  |
4 |     let total = (price * cuont);
  |                          ^^^^^
  |
help: a local variable with a similar name exists
  |
4 -     let total = (price * cuont);
4 +     let total = (price * count);
  |

warning: unnecessary parentheses around assigned value
 --> src/main.rs:4:17
  |
4 |     let total = (price * cuont);
  |                 ^             ^
  |
  = note: `#[warn(unused_parens)]` (part of `#[warn(unused)]`) on by default
help: remove these parentheses
  |
4 -     let total = (price * cuont);
4 +     let total = price * cuont;
  |

error[E0308]: mismatched types
 --> src/main.rs:2:22
  |
2 |     let price: u32 = 2.5;
  |                ---   ^^^ expected `u32`, found floating-point number
  |                |
  |                expected due to this

error[E0308]: mismatched types
 --> src/main.rs:3:22
  |
3 |     let count: u32 = "three";
  |                ---   ^^^^^^^ expected `u32`, found `&str`
  |                |
  |                expected due to this

Some errors have detailed explanations: E0308, E0425.
For more information about an error, try `rustc --explain E0308`.
warning: `hello` (bin "hello") generated 1 warning
error: could not compile `hello` (bin "hello") due to 3 previous errors; 1 warning emitted
"#;

/// A. Read FIRST_MESSAGE. Return its error code, for example `"E0123"`.
fn first_error_code() -> &'static str {
    todo!("A: the code in the square brackets")
}

/// B. Read FIRST_MESSAGE. Return the line number and the column number at
/// which the problem starts, as a pair: `(line, column)`.
fn first_error_position() -> (u32, u32) {
    todo!("B: the two numbers after the file name")
}

/// C. Read FIRST_MESSAGE. The compiler proposes a repair. Return the name
/// that should stand in the program in place of the misspelled one.
fn first_error_repair() -> &'static str {
    todo!("C: the name from the help section")
}

/// D. Read SECOND_MESSAGE. Return the type that the compiler expected and the
/// type that it found, in that order, as two texts: `("...", "...")`.
/// Write the types as the message prints them, without the backticks.
fn second_error_types() -> (&'static str, &'static str) {
    todo!("D: (expected, found)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::guess;

    // The helpers below read the answers out of the compiler messages, so
    // that the tests do not contain them. You do not need to read the helpers.

    /// The text between `start` and the next `end`, searched from the top.
    fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
        let from = text.find(start).expect("marker not found") + start.len();
        let length = text[from..].find(end).expect("end marker not found");
        &text[from..from + length]
    }

    /// The line and column of the first `-->` arrow in `text`.
    fn first_arrow(text: &str) -> (u32, u32) {
        let place = between(text, "--> src/main.rs:", "\n");
        let (line, column) = place.split_once(':').expect("line:column");
        (line.parse().unwrap(), column.parse().unwrap())
    }

    #[test]
    fn a_error_code_is_read_from_the_headline() {
        assert_eq!(
            first_error_code(),
            between(FIRST_MESSAGE, "error[", "]"),
            "A: the code stands in square brackets after the word `error` \
             (left: your answer, right: the code in the message)"
        );
    }

    #[test]
    fn b_position_is_read_from_the_arrow_line() {
        assert_eq!(
            first_error_position(),
            first_arrow(FIRST_MESSAGE),
            "B: the line with `-->` ends in file:line:column \
             (left: your answer, right: the pair in the message)"
        );
    }

    #[test]
    fn c_repair_is_read_from_the_help() {
        let help = between(FIRST_MESSAGE, "help:", "For more information");
        let proposed_line = help
            .lines()
            .find(|line| line.contains("println!"))
            .expect("the help shows the repaired line");
        let name = between(proposed_line, "{}\", ", ")");
        assert_eq!(
            first_error_repair(),
            name,
            "C: the help section shows the repaired line; return the name it uses \
             (left: your answer, right: the name in the help)"
        );
    }

    #[test]
    fn d_expected_and_found_types() {
        let label = between(SECOND_MESSAGE, "^ expected ", "\n");
        let expected = between(label, "`", "`");
        let found = between(label, "found `", "`");
        assert_eq!(
            second_error_types(),
            (expected, found),
            "D: the label under the `^^^` marks says `expected ..., found ...` \
             (left: your answer, right: the two types in the message)"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    ///
    /// Both predictions are about LONG_OUTPUT. Read it from the top.
    #[test]
    fn p_predictions() {
        // P1. How many errors and how many warnings does LONG_OUTPUT report
        // about the program? Do not count the closing lines that only sum up.
        // Answer as (errors, warnings).
        let errors = LONG_OUTPUT
            .lines()
            .filter(|line| line.starts_with("error["))
            .count();
        let warnings = LONG_OUTPUT
            .lines()
            .filter(|line| line.starts_with("warning: ") && !line.contains("generated"))
            .count();
        let p1: (usize, usize) = guess();
        assert_eq!(
            p1,
            (errors, warnings),
            "P1: (errors, warnings) in LONG_OUTPUT (left: your prediction, right: actual)"
        );

        // P2. You repair the FIRST error that the compiler printed. Which line
        // of src/main.rs do you open?
        let p2: u32 = guess();
        assert_eq!(
            p2,
            first_arrow(LONG_OUTPUT).0,
            "P2: the line of the first reported error (left: your prediction, right: actual)"
        );
    }
}
