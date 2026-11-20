//! LESSON 11 — Capstone: a lazy measurement report
//! Run: cargo test --test chapter15 l11_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: How many lines are read when limit=0, and
//! when the last wanted row arrives late? Which adapters skip work?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why must enumerate precede trimming/filtering to preserve original
//! source numbering? Show where each consumer starts execution.
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch15_scratch.rs):
//! Replace the final collect of task B with a checked try_fold that fails on the first
//! invalid line instead of skipping it, and explain the changed error policy.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, and explanation are done.

#[derive(Debug, PartialEq)]
struct Report {
    rows: Vec<(usize, i32)>,
    sum: i64,
    mean: Option<f64>,
}
/// A. Return one lazy pipeline over `lines` that yields (line number, value) rows.
/// Ignore blank lines, comment lines (trimmed text starts with '#') and text that
/// is not a valid i32 after trimming. Keep values >= minimum, keep the original
/// 1-based line numbers, and stop after `limit` rows.
/// No intermediate collections. Nothing is read when the pipeline is built, and no
/// line is read after the last wanted row has been found.
fn rows<'a>(
    lines: impl Iterator<Item = &'a str>,
    minimum: i32,
    limit: usize,
) -> impl Iterator<Item = (usize, i32)> {
    lines
        .enumerate() // number first, while every source line is still present
        .filter_map(|(index, line)| {
            let text = line.trim();
            // Blank lines and comments would fail to parse anyway; the explicit
            // test states the rule from the contract.
            if text.is_empty() || text.starts_with('#') {
                return None;
            }
            text.parse::<i32>().ok().map(|value| (index + 1, value))
        })
        .filter(move |&(_, value)| value >= minimum)
        .take(limit) // last, so that nothing is read after the final row
}
/// B. Collect the rows of a slice of lines, using `rows`.
fn selected_rows(lines: &[&str], minimum: i32, limit: usize) -> Vec<(usize, i32)> {
    rows(lines.iter().copied(), minimum, limit).collect()
}
/// C. Assemble a report from the selected rows; preserve order. `sum` adds the
/// values as i64, and `mean` is None when there are no rows.
fn report(lines: &[&str], minimum: i32, limit: usize) -> Report {
    let rows = selected_rows(lines, minimum, limit);
    let sum: i64 = rows.iter().map(|&(_, value)| i64::from(value)).sum();
    // Dividing by a zero count would give NaN, so the empty case is None.
    let mean = (!rows.is_empty()).then(|| sum as f64 / rows.len() as f64);
    Report { rows, sum, mean }
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::cell::Cell;
    #[test]
    fn a_rows_keep_source_line_numbers() {
        let lines = [" # comment", "3", "oops", " 9 ", "-2", "9", ""];
        let all: Vec<(usize, i32)> = rows(lines.iter().copied(), 3, 9).collect();
        assert_eq!(
            all,
            [(2, 3), (4, 9), (6, 9)],
            "number the lines before anything is filtered out, starting at 1"
        );
        // `rows` accepts any iterator of &str, for example `str::lines`.
        let text = "5\n# skip\n\n6";
        assert_eq!(rows(text.lines(), 6, 9).collect::<Vec<_>>(), [(4, 6)]);
    }
    #[test]
    fn a_rows_read_nothing_until_asked_and_nothing_after_the_limit() {
        let source: Vec<String> = (0..1000).map(|i| i.to_string()).collect();
        let pulled = Cell::new(0);
        let lines = source
            .iter()
            .map(String::as_str)
            .inspect(|_| pulled.set(pulled.get() + 1));
        let mut pipeline = rows(lines, 10, 3);
        assert_eq!(
            pulled.get(),
            0,
            "building the pipeline must not read any line; it read {}",
            pulled.get()
        );
        assert_eq!(pipeline.next(), Some((11, 10)));
        assert_eq!(
            pulled.get(),
            11,
            "the first row is on line 11, so the first `next` reads 11 lines"
        );
        assert_eq!(pipeline.by_ref().collect::<Vec<_>>(), [(12, 11), (13, 12)]);
        assert_eq!(pipeline.next(), None);
        assert_eq!(
            pulled.get(),
            13,
            "the third and last wanted row is on line 13; no later line may be read, \
             but {} lines were read",
            pulled.get()
        );
    }
    #[test]
    fn a_rows_with_limit_zero_read_nothing() {
        let pulled = Cell::new(0);
        let lines = ["1", "2", "3"]
            .into_iter()
            .inspect(|_| pulled.set(pulled.get() + 1));
        assert_eq!(rows(lines, 0, 0).count(), 0);
        assert_eq!(pulled.get(), 0, "a limit of 0 needs no input at all");
    }
    #[test]
    fn b_selected_rows_keep_source_line_numbers_and_limit() {
        let lines = [" # comment", "3", "oops", " 9 ", "-2", "9", ""];
        assert_eq!(selected_rows(&lines, 3, 2), [(2, 3), (4, 9)]);
        assert_eq!(selected_rows(&lines, 3, 9), [(2, 3), (4, 9), (6, 9)]);
        assert!(selected_rows(&lines, 0, 0).is_empty());
        assert_eq!(selected_rows(&["#5", "  ", "+7"], 7, 9), [(3, 7)]);
    }
    #[test]
    fn b_selected_rows_collect_once() {
        let lines: Vec<String> = (0..1000).map(|i| i.to_string()).collect();
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        let (rows, allocations) = count(|| selected_rows(&lines, 10, 3));
        assert_eq!(rows, [(11, 10), (12, 11), (13, 12)]);
        assert!(
            allocations <= 1,
            "only the output vector should allocate, but there were {allocations} \
             allocations; look for a collect in the middle of the pipeline"
        );
    }
    #[test]
    fn c_report_combines_owned_collection_and_consumers() {
        assert_eq!(
            report(&["3", "bad", "5"], 0, 10),
            Report {
                rows: vec![(1, 3), (3, 5)],
                sum: 8,
                mean: Some(4.0)
            }
        );
        assert_eq!(
            report(&[], 0, 10),
            Report {
                rows: vec![],
                sum: 0,
                mean: None
            },
            "no rows means no mean: None, not NaN"
        );
        let r = report(&["2147483647", "2147483647"], 0, 10);
        assert_eq!(r.sum, 4_294_967_294, "add the values as i64, not as i32");
        assert_eq!(r.mean, Some(2_147_483_647.0));
        assert_eq!(report(&["1", "2", "#9"], 0, 10).mean, Some(1.5));
        assert_eq!(report(&["1", "2", "3"], 0, 2).sum, 3);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Lines read from the source: (when the pipeline ends in take(0),
        // when it ends in take(2)).
        let lines = ["4", "x", "7", "9", "1"];
        let read = Cell::new(0);
        let lines_read = |limit: usize| {
            read.set(0);
            let _kept: Vec<i32> = lines
                .iter()
                .inspect(|_| read.set(read.get() + 1))
                .filter_map(|s| s.parse().ok())
                .take(limit)
                .collect();
            read.get()
        };
        let p1: (usize, usize) = (0, 3);
        assert_eq!(
            p1,
            (lines_read(0), lines_read(2)),
            "P1: lines read (with take(0), with take(2))"
        );

        // P2. The numbers attached to "a" and "b": (enumerate first, filter first).
        let text = ["", "a", "", "b"];
        let numbered_first: Vec<usize> = text
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.is_empty())
            .map(|(i, _)| i)
            .collect();
        let filtered_first: Vec<usize> = text
            .iter()
            .filter(|s| !s.is_empty())
            .enumerate()
            .map(|(i, _)| i)
            .collect();
        let p2: (Vec<usize>, Vec<usize>) = (vec![1, 3], vec![0, 1]);
        assert_eq!(
            p2,
            (numbered_first, filtered_first),
            "P2: indices of a and b (enumerate before filter, filter before enumerate)"
        );

        // P3. A mean computed without checking for "no rows":
        // (is it NaN?, is it below 1.0?, is it at least 1.0?)
        let none: [f64; 0] = [];
        let mean = none.iter().sum::<f64>() / none.len() as f64;
        let p3: (bool, bool, bool) = (true, false, false);
        assert_eq!(
            p3,
            (mean.is_nan(), mean < 1.0, mean >= 1.0),
            "P3: (mean is NaN, mean < 1.0, mean >= 1.0)"
        );
    }
}
