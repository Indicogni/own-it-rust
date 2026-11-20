//! LESSON 05 — Attributes, cfg and included files
//! Run: cargo test --test chapter08 l05_
//!
//! Read first: CONCEPTS.md, section "Lesson 05".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider: Is code behind
//! `#[cfg(false)]` type-checked? Does `if cfg!(false) { .. }` remove the
//! unselected branch from type checking?
//!
//! IMPLEMENT the tasks below. Keep the tests and add one edge case.
//!
//! EXPLAIN afterward: Explain conditional compilation versus runtime
//! branching, environment capture at compile time, and file inclusion paths.
//!
//! LAB (optional): LABS.md, Lab 4, "cfg in the scratch file". The checked lab
//! in label_lab/ has you declare a Cargo feature (step 6) and write a build
//! script (step 7).

static EMBEDDED: &str = include_str!("included_labels.txt");
/// A. Return whether this target belongs to the Unix family using cfg!.
fn unix_target() -> bool {
    cfg!(target_family = "unix")
}
/// B. Borrow nonempty trimmed lines from the embedded file, in order. No
/// per-line String allocation; returned text has static lifetime.
fn embedded_labels() -> Vec<&'static str> {
    // Every item is a sub-slice of EMBEDDED, so only the Vec allocates.
    EMBEDDED
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_target_attribute_is_compile_time_information() {
        assert_eq!(unix_target(), cfg!(target_family = "unix"));
    }
    #[test]
    fn b_file_is_embedded_and_borrowed() {
        assert_eq!(embedded_labels(), ["north", "港", "south"]);
        let start = EMBEDDED.as_ptr() as usize;
        for label in embedded_labels() {
            let p = label.as_ptr() as usize;
            assert!(
                p >= start && p + label.len() <= start + EMBEDDED.len(),
                "{label:?} must be a slice of EMBEDDED, not a copy or a literal"
            );
        }
    }
    #[test]
    fn b_labels_need_no_per_line_allocation() {
        let (labels, allocations) = count(embedded_labels);
        assert_eq!(labels.len(), 3);
        assert!(
            allocations <= 2,
            "only the Vec may allocate; got {allocations}"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// Open included_labels.txt if you like.
    #[test]
    fn p_predictions() {
        // P1. How many lines does str::lines() report for the embedded file?
        let p1: usize = 4;
        assert_eq!(p1, EMBEDDED.lines().count(), "P1: blank lines count too");

        // P2. Today you write cfg!(true) and cfg!(false). Older code spells
        // the two constants as an empty all() and an empty any(). Which is
        // which?
        let p2: (bool, bool, bool) = (true, false, true);
        assert_eq!(
            p2,
            (cfg!(all()), cfg!(any()), cfg!(not(any()))),
            "P2: all of nothing holds; any of nothing does not"
        );

        // P3. #[cfg] on statements removes them before compilation.
        let mut kept = String::new();
        #[cfg(not(test))]
        kept.push('a');
        kept.push('b');
        #[cfg(test)]
        kept.push('c');
        #[cfg(target_pointer_width = "64")]
        kept.push('d');
        let p3: &str = "bcd";
        assert_eq!(p3, kept, "P3: attributes on statements");
    }
}
