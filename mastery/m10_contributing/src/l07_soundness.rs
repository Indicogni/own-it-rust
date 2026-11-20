//! LESSON 07 — Unsafe code and soundness reports
//! Run: cargo test --manifest-path mastery/m10_contributing/Cargo.toml l07_
//! LAB: cargo +nightly-2026-11-20 miri test --manifest-path mastery/m10_contributing/Cargo.toml --lib l07_soundness::tests::b_lab
//!      Keep the filter: Miri isolates a program from the file system, so the tests
//!      of this package that read files in `answers/` cannot run under it.
//! Read first: CONCEPTS.md, "Lesson 07". The poor report in
//! `fixture/issues/0029-slice-prints-garbage.md` is your only lead.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: What does it mean that a *safe*
//! function is unsound? Why can an unsound function pass every ordinary test?
//! IMPLEMENT: A. find the unsound function in the fixture. B. give arguments with
//! which safe code reaches undefined behaviour, and watch Miri report it (the LAB).
//! C. fix the function in the fixture without changing what it returns for valid
//! arguments. D. write the advisory in `answers/l07_advisory.md`. Preserve the tests.
//! EXPLAIN afterward: State the safety requirement of the `unsafe` block as it was
//! written, the requirement it really has, and which of the two problems of the
//! function Miri can see and which it cannot.

/// A. The unsound function. Give its path as rustdoc prints it, for example
/// `tasklane_core::glob::matches` or `tasklane_plugin::Workspace::write`.
pub fn unsound_function() -> &'static str {
    todo!("Implement this task")
}

/// A call of the unsound function: a file with this content, and the two numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exposure {
    /// The content of the file.
    pub content: &'static str,
    /// The second argument of the call.
    pub start: usize,
    /// The third argument of the call.
    pub len: usize,
}

/// B. Arguments with which the function, as released in 0.4.0, passes its own check
/// and then builds a slice that leaves the file's memory. Any safe caller can pass
/// them; that is what makes the function unsound.
pub fn exposure() -> Exposure {
    todo!("Implement this task")
}

/// Makes the call described by [`exposure`] and says whether a slice came back.
/// This function contains no `unsafe`. Do not edit it.
pub fn make_the_call() -> bool {
    let exposure = exposure();
    // The standard library aborts the whole process, not only one test, when a debug
    // build asks for a slice of more than `isize::MAX` bytes. Stay far below that.
    assert!(
        exposure.len <= 1 << 20,
        "B: keep `len` below one megabyte; one byte too many is enough to show the problem"
    );
    let mut ws = tasklane_plugin::Workspace::new();
    ws.write("file", exposure.content);
    ws.read_range("file", exposure.start, exposure.len)
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{assert_answer, read, require_section};
    // Unused once every prediction is answered.
    #[allow(unused_imports)]
    use crate::guess;
    use tasklane_plugin::Workspace;

    #[test]
    fn a_unsound_function() {
        assert_answer("A", unsound_function(), 0x47ea_6623_4e00_736f);
    }

    #[test]
    fn b_exposure_gets_past_the_old_check_and_leaves_the_file() {
        let e = exposure();
        assert!(
            e.start <= e.content.len(),
            "B: version 0.4.0 returns `None` for this `start`, so the `unsafe` block is never reached"
        );
        let leaves_the_file = e
            .start
            .checked_add(e.len)
            .is_none_or(|end| end > e.content.len());
        assert!(
            leaves_the_file,
            "B: bytes {}..{} lie inside a file of {} bytes; this call is fine. The \
             arguments must make the slice end after the last byte of the file.",
            e.start,
            e.start + e.len,
            e.content.len()
        );
    }

    /// The LAB test. Under an ordinary `cargo test` it only checks the result of the
    /// call. Under Miri (see the LAB line at the top) the same call is checked
    /// operation by operation: before your fix Miri stops with "Undefined Behavior",
    /// after it the test passes.
    ///
    /// Before the fix this call builds a slice that leaves the file's memory. The
    /// test never reads through that slice.
    #[test]
    fn b_lab_exposing_call_returns_none_after_the_fix() {
        assert!(
            !make_the_call(),
            "B/C: `read_range` returned a slice for a range that is not inside the file"
        );
    }

    fn file(content: &str) -> Workspace {
        let mut ws = Workspace::new();
        ws.write("a", content);
        ws
    }

    // The `c_fixed_` tests never read through a slice that the unfixed function
    // returns: they only ask whether it is `Some`.

    #[test]
    fn c_fixed_ranges_outside_the_file_give_none() {
        let ws = file("hello");
        // The order matters: the last case is only reached once the simple ones pass.
        for (start, len) in [
            (0, 6),
            (5, 1),
            (4, 2),
            (6, 0),
            (usize::MAX, 2),
            (2, usize::MAX),
        ] {
            assert!(
                ws.read_range("a", start, len).is_none(),
                "C: read_range(\"a\", {start}, {len}) on a 5-byte file must be None"
            );
        }
    }

    #[test]
    fn c_fixed_ranges_that_split_a_character_give_none() {
        // "h", then a two-byte character, then "llo": 6 bytes.
        let ws = file("h\u{e9}llo");
        for (start, len) in [(0, 2), (2, 1), (2, 4), (1, 1)] {
            assert!(
                ws.read_range("a", start, len).is_none(),
                "C: read_range(\"a\", {start}, {len}) would cut a character in two. A \
                 `&str` must be valid UTF-8, so the answer must be None."
            );
        }
    }

    #[test]
    fn c_unchanged_valid_ranges_still_borrow_from_the_file() {
        let ws = file("hello, w\u{f6}rld");
        assert_eq!(ws.read_range("a", 0, 5), Some("hello"));
        assert_eq!(ws.read_range("a", 7, 6), Some("w\u{f6}rld"));
        assert_eq!(ws.read_range("a", 8, 2), Some("\u{f6}"));
        assert_eq!(ws.read_range("a", 13, 0), Some(""));
        assert_eq!(ws.read_range("a", 0, 0), Some(""));
        assert_eq!(ws.read_range("missing", 0, 0), None);
        let whole = ws.read("a").unwrap();
        let part = ws.read_range("a", 7, 6).unwrap();
        assert!(
            std::ptr::eq(part.as_ptr(), whole[7..].as_ptr()),
            "C: the result must still borrow from the file, not be a copy"
        );
    }

    #[test]
    fn d_advisory_has_every_section() {
        let file = "answers/l07_advisory.md";
        let advisory = read(file);
        let title = advisory
            .lines()
            .find_map(|line| line.strip_prefix("# "))
            .expect("D: the advisory starts with a `# Title` line");
        assert!(
            title.contains("read_range"),
            "D: the title must name the affected function"
        );
        let affected = require_section(file, &advisory, "## Affected versions");
        assert!(
            affected.contains("0.4.0"),
            "D: say which released version is affected"
        );
        let description = require_section(file, &advisory, "## Description");
        assert!(
            description.split_whitespace().count() >= 40,
            "D: describe the problem in at least a few sentences"
        );
        assert!(
            description.to_lowercase().contains("safe"),
            "D: the description must say that safe code can trigger the problem; that \
             is the difference between a bug in `unsafe` code and an unsound API"
        );
        require_section(file, &advisory, "## Impact");
        let reproduction = require_section(file, &advisory, "## Reproduction");
        assert!(
            reproduction.contains("```") && reproduction.contains("read_range"),
            "D: `## Reproduction` needs a code block that calls the function"
        );
        assert!(
            reproduction.contains("Miri") || reproduction.contains("miri"),
            "D: say how to observe the problem; an ordinary run usually shows nothing"
        );
        let fix = require_section(file, &advisory, "## Fix");
        assert!(
            fix.contains("0.4.1"),
            "D: name the version that will contain the fix (the next patch release)"
        );
        require_section(file, &advisory, "## Workaround");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. "h\u{e9}llo" is h, e with an acute accent, l, l, o. Its `len()`?
        let p1: usize = guess();
        let text = "h\u{e9}llo";
        assert_eq!(p1, text.len(), "P1: `len` counts bytes, not characters");

        // P2. text.is_char_boundary(1), text.is_char_boundary(2), text.is_char_boundary(6)
        let p2: (bool, bool, bool) = guess();
        assert_eq!(
            p2,
            (
                text.is_char_boundary(1),
                text.is_char_boundary(2),
                text.is_char_boundary(6)
            ),
            "P2: where a `&str` may be cut"
        );

        // P3. Safe slicing with `get`: is each of these `Some`?
        //     "abc".get(1..3), "abc".get(1..9), "abc".get(3..3), text.get(1..2)
        let p3: [bool; 4] = guess();
        assert_eq!(
            p3,
            [
                "abc".get(1..3).is_some(),
                "abc".get(1..9).is_some(),
                "abc".get(3..3).is_some(),
                text.get(1..2).is_some()
            ],
            "P3: `str::get` checks the bounds and the character boundaries"
        );

        // P4. 5usize.checked_add(usize::MAX).is_some()
        let p4: bool = guess();
        assert_eq!(
            p4,
            5usize.checked_add(usize::MAX).is_some(),
            "P4: `start + len` can overflow before any bounds check sees it"
        );
    }
}
