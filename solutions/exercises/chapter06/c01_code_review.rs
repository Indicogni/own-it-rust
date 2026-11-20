//! CODE REVIEW 01 — A teammate's pull request
//! Run: cargo test --test chapter06 c01_
//! Then: cargo clippy --test chapter06
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! and most behavior tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug, every needless allocation, and every statement-style
//!    loop or flag that an expression would say more clearly.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. You MAY change parameter types where the doc comment says so;
//!    the tests are written to accept the better signature.
//! 4. Run clippy and resolve its warnings for this file.

/// Return the 1-based line number of the first line whose trimmed text is
/// "end", or None. Stop at that line: later lines need no inspection (the
/// tests cannot observe this, so check it in your review). You may change the
/// parameter type to `&[String]`.
fn end_line(lines: &[String]) -> Option<usize> {
    // position stops at the first match, and trim() already gives a &str.
    lines
        .iter()
        .position(|line| line.trim() == "end")
        .map(|index| index + 1)
}

/// Classify a status code: 100..=199 "info", 200..=299 "ok",
/// 300..=399 "redirect", 400..=599 "error", anything else "invalid".
/// Keep this signature.
fn classify(code: u16) -> &'static str {
    match code {
        100..=199 => "info",
        200..=299 => "ok",
        300..=399 => "redirect",
        400..=599 => "error",
        _ => "invalid",
    }
}

/// Sum byte counts written as decimal text (surrounding whitespace allowed).
/// None if any entry is not a valid number or the total does not fit in u32.
/// Keep this signature.
fn total_size(sizes: &[&str]) -> Option<u32> {
    let mut total: u32 = 0;
    for size in sizes {
        let Ok(bytes) = size.trim().parse::<u32>() else {
            return None;
        };
        // `?` on an Option returns None early (Chapter 7). It is shorthand for
        // `let Some(sum) = total.checked_add(bytes) else { return None; };`.
        total = total.checked_add(bytes)?;
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, within};

    fn lines(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_end_line_is_one_based_and_first() {
        // `within` fails the test with a message if your loop never ends.
        within(10, "the loop in end_line", || {
            assert_eq!(end_line(&lines(&["a", " end ", "end"])), Some(2));
            assert_eq!(end_line(&lines(&["end"])), Some(1));
            assert_eq!(end_line(&lines(&["ending", "END"])), None);
            assert_eq!(end_line(&Vec::<String>::new()), None);
        });
    }

    #[test]
    fn a_end_line_borrows_instead_of_copying() {
        // Allocations are counted per thread, so `count` runs inside `within`.
        let (found, allocations) = within(10, "the loop in end_line", || {
            let mut many = lines(&["x"; 200]);
            many[3] = "end".into();
            count(|| end_line(&many))
        });
        assert_eq!(found, Some(4));
        assert_eq!(allocations, 0, "comparing &str needs no new String");
    }

    #[test]
    fn b_classify_covers_every_range_edge() {
        for (code, kind) in [
            (99, "invalid"),
            (100, "info"),
            (199, "info"),
            (200, "ok"),
            (299, "ok"),
            (300, "redirect"),
            (399, "redirect"),
            (400, "error"),
            (599, "error"),
            (600, "invalid"),
            (0, "invalid"),
            (u16::MAX, "invalid"),
        ] {
            assert_eq!(classify(code), kind, "code {code}");
        }
    }

    #[test]
    fn c_total_size_adds_valid_entries() {
        assert_eq!(total_size(&[" 10", "20 ", "0"]), Some(30));
        assert_eq!(total_size(&[]), Some(0));
        assert_eq!(total_size(&["10", "ten"]), None);
        assert_eq!(total_size(&["-1"]), None);
    }

    #[test]
    fn c_total_size_rejects_overflow_instead_of_truncating() {
        assert_eq!(total_size(&["4294967295"]), Some(u32::MAX));
        assert_eq!(total_size(&["4294967295", "1"]), None);
        assert_eq!(total_size(&["4294967296"]), None);
        assert_eq!(total_size(&["18446744073709551615", "1"]), None);
    }
}
