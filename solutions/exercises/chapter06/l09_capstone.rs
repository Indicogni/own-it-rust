//! LESSON 09 — Capstone: command expression engine
//! Run: cargo test --test chapter06 l09_
//! Read first: CONCEPTS.md, section "Lesson 09".
//!
//! PREDICT before running: Which commands should stop a loop, skip a line, or
//! return an error immediately? Where should the error line number come from?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain the role of each expression construct, how every
//! path produces Result, and why work after stop is not inspected.
//! COMPILER / TRANSFER LAB: Extend the language with reset. Before coding, decide
//! whether reset is a command or a value and whether it changes error line
//! numbering. Rewrite one let else as match and compare clarity.
//! Use examples/ch06_scratch.rs; restore it to compiling code afterwards.

/// A. Trim one line, then decode it: blank/comment => Ok(None), an i32 as
/// str::parse reads it => Ok(Some(i32)), anything else => Err(()). A comment
/// line starts with '#' after trimming; there are no trailing comments, so
/// "4 # four" is invalid.
fn decode(line: &str) -> Result<Option<i32>, ()> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }
    match line.parse::<i32>() {
        Ok(value) => Ok(Some(value)),
        Err(_) => Err(()),
    }
}
/// B. Sum decoded lines until a trimmed "stop". Return Err(one_based_line)
/// for invalid input OR i32 sum overflow, ignoring all lines after stop.
/// Blank and comment lines still count toward line numbering. Check overflow
/// after every addition (not only at the end), and allocate nothing: walk the
/// borrowed lines with a loop; do not collect them or build Strings.
fn execute(lines: &[&str]) -> Result<i32, usize> {
    let mut sum = 0i32;
    // enumerate counts every line, so skipped lines keep their numbers.
    for (index, line) in lines.iter().enumerate() {
        let number = index + 1;
        if line.trim() == "stop" {
            break;
        }
        let Ok(decoded) = decode(line) else {
            return Err(number);
        };
        let Some(value) = decoded else {
            continue;
        };
        // checked_add reports the overflow on the line where it happens.
        let Some(next) = sum.checked_add(value) else {
            return Err(number);
        };
        sum = next;
    }
    Ok(sum)
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, within};
    #[test]
    fn a_decode_separates_skips_from_errors() {
        assert_eq!(decode(" -3 "), Ok(Some(-3)));
        assert_eq!(decode(" # note"), Ok(None));
        assert_eq!(decode("  "), Ok(None));
        assert_eq!(decode("bad"), Err(()));
        assert_eq!(decode("#"), Ok(None));
        assert_eq!(decode("+7"), Ok(Some(7)));
        assert_eq!(decode("4 # four"), Err(()));
        assert_eq!(decode("2147483648"), Err(()));
        assert_eq!(decode("\u{3000}12\u{3000}"), Ok(Some(12)));
    }
    #[test]
    fn b_engine_controls_evaluation() {
        // `within` fails the test with a message if your loop never ends.
        within(10, "the loop in execute", || {
            assert_eq!(execute(&["# h", "2", "-1", " stop ", "bad"]), Ok(1));
            assert_eq!(execute(&["", "bad"]), Err(2));
            assert_eq!(execute(&["2147483647", "1"]), Err(2));
            assert_eq!(execute(&[]), Ok(0));
            assert_eq!(execute(&["stop", "2147483647", "1"]), Ok(0));
            assert_eq!(execute(&["-2147483648", "-1"]), Err(2));
            assert_eq!(execute(&["1", "STOP"]), Err(2), "stop is case-sensitive");
        });
    }
    #[test]
    fn b_line_numbers_count_every_line() {
        within(10, "the loop in execute", || {
            assert_eq!(
                execute(&["# a", "", "1", "  ", "x"]),
                Err(5),
                "blank and comment lines still count toward the line number"
            );
            assert_eq!(execute(&["", "", "stop", "x"]), Ok(0));
        });
    }
    #[test]
    fn b_overflow_is_checked_at_every_step() {
        within(10, "the loop in execute", || {
            // A wider running total would end at i32::MAX and miss the overflow on line 2.
            assert_eq!(
                execute(&["2147483647", "1", "-1"]),
                Err(2),
                "the total leaves the i32 range on line 2, even though it comes back later"
            );
            assert_eq!(execute(&["2147483647", "-1", "1"]), Ok(i32::MAX));
        });
    }
    #[test]
    fn a_b_engine_does_not_allocate() {
        // Allocations are counted per thread, so `count` runs inside `within`.
        let (sum, allocations) = within(10, "the loop in execute", || {
            let lines: Vec<String> = (0..100).map(|i| format!(" {i} ")).collect();
            let mut borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
            borrowed.push("# done");
            count(|| execute(&borrowed))
        });
        assert_eq!(sum, Ok(4950));
        assert_eq!(
            allocations, 0,
            "execute must not allocate: walk the borrowed lines and build no String or Vec"
        );
    }
}
