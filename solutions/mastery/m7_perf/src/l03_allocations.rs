//! LESSON 03 — Allocation budgets: capacity, reuse, borrowing, and lazy ownership
//! Read first: CONCEPTS.md, "Lesson 03". Start after: lesson 02; chapters 16
//! (collections) and 17 (strings), and `Cow` from chapter 13.
//! Run: cargo test --manifest-path mastery/m7_perf/Cargo.toml l03_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a function builds a `String` of
//! 1,000 bytes by pushing one byte at a time into `String::new()`. Roughly how
//! many allocations does it make, and how many bytes does it ask the allocator
//! for in total?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! Every task has a budget: a largest number of allocations, and sometimes of
//! bytes. The tests measure with `support::alloc::measure` and fail with the
//! numbers when you are over.
//! EXPLAIN afterward: For each task, say where the naive version allocates and
//! which of these removed it: knowing the size in advance, reusing a buffer,
//! borrowing instead of owning, or owning only when needed. When is an
//! allocation not worth removing?
//! JUDGMENT: An allocation costs tens of nanoseconds when the allocator's fast
//! path is hit, and much more under contention or when the block is large.
//! That matters in a loop that runs a million times and not at all in code
//! that runs once per request next to a database call. Remove allocations
//! where a profile shows them, and then keep them out with a budget test like
//! the ones here. Crates such as `smallvec`, `bumpalo` and `compact_str` go
//! further (inline storage, arenas); reach for them when a profile still shows
//! the allocator after the changes of this lesson.

use std::borrow::Cow;
use std::collections::HashMap;

/// A. Render one row of a CSV file, without a line ending.
///
/// Fields are separated by `,`. A field that contains a comma, a double quote
/// or a line break is written inside double quotes, and each double quote in
/// it is doubled: `say "hi"` becomes `"say ""hi"""`. Other fields are written
/// as they are. No fields give an empty string.
///
/// Budget: at most one allocation, and the bytes asked for must equal the
/// length of the result. Find the length first, then write.
pub fn csv_row(fields: &[&str]) -> String {
    let mut row = String::with_capacity(csv_row_len(fields));
    push_csv_row(fields, &mut row);
    row
}

fn needs_quotes(field: &str) -> bool {
    field.contains([',', '"', '\n', '\r'])
}

/// The length in bytes of the row that `push_csv_row` writes.
fn csv_row_len(fields: &[&str]) -> usize {
    let separators = fields.len().saturating_sub(1);
    let content: usize = fields
        .iter()
        .map(|field| {
            if needs_quotes(field) {
                // Two quotes around the field and one more for each quote in it.
                field.len() + 2 + field.matches('"').count()
            } else {
                field.len()
            }
        })
        .sum();
    separators + content
}

/// Appends the row to `out`. It never builds a temporary string, so the only
/// allocation it can cause is growth of `out`.
fn push_csv_row(fields: &[&str], out: &mut String) {
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        if needs_quotes(field) {
            out.push('"');
            for piece in field.split_inclusive('"') {
                out.push_str(piece);
                if piece.ends_with('"') {
                    out.push('"');
                }
            }
            out.push('"');
        } else {
            out.push_str(field);
        }
    }
}

/// B. Append every row to `out` in the format of task A, each followed by
/// `\n`. Keep what `out` already holds.
///
/// Budget: no allocation at all when `out` has enough spare capacity. That
/// rules out building a `String` per row. When `out` is too small, grow it at
/// most once.
pub fn csv_rows_into(rows: &[Vec<&str>], out: &mut String) {
    let needed: usize = rows.iter().map(|row| csv_row_len(row) + 1).sum();
    // `reserve` does nothing when the spare capacity is already enough.
    out.reserve(needed);
    for row in rows {
        push_csv_row(row, out);
        out.push('\n');
    }
}

/// C. Replace every run of two or more spaces by one space.
///
/// Budget: an input without such a run is returned borrowed, with no
/// allocation. Otherwise allocate at most once.
pub fn squeeze_spaces(input: &str) -> Cow<'_, str> {
    let Some(first_run) = input.find("  ") else {
        return Cow::Borrowed(input);
    };
    // The result is shorter than the input, so this capacity is enough.
    let mut squeezed = String::with_capacity(input.len());
    // Everything before the first run is copied in one piece.
    squeezed.push_str(&input[..first_run]);
    let mut last_was_space = false;
    for c in input[first_run..].chars() {
        if c != ' ' || !last_was_space {
            squeezed.push(c);
        }
        last_was_space = c == ' ';
    }
    Cow::Owned(squeezed)
}

/// D. Count how often each word occurs. The result owns its keys, because it
/// outlives `words`.
///
/// Budget: one `String` per *distinct* word, plus the growth of the map. A
/// word that is in the map already must not cost an allocation.
pub fn tally(words: &[&str]) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for &word in words {
        // `entry` would need an owned key before it knows whether the word is
        // new. `get_mut` looks up with the borrowed `&str`; only a miss pays
        // for a `String`.
        match counts.get_mut(word) {
            Some(count) => *count += 1,
            None => {
                counts.insert(word.to_owned(), 1);
            }
        }
    }
    counts
}

/// E. The median of every row: sort a copy of the row and take the element at
/// index `len / 2`. An empty row has the median 0. The rows must stay as they
/// are.
///
/// Budget: at most two allocations in total, however many rows there are and
/// however long they are: the result, and one scratch buffer that every row
/// reuses. The sort must not allocate either. `sort` (the stable sort) asks
/// for scratch memory of its own when a slice is long; `sort_unstable` sorts
/// in place and never allocates.
pub fn row_medians(rows: &[Vec<u32>]) -> Vec<u32> {
    let longest = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut scratch: Vec<u32> = Vec::with_capacity(longest);
    let mut medians = Vec::with_capacity(rows.len());
    for row in rows {
        // `clear` keeps the capacity, so this copy never allocates.
        scratch.clear();
        scratch.extend_from_slice(row);
        scratch.sort_unstable();
        medians.push(scratch.get(scratch.len() / 2).copied().unwrap_or(0));
    }
    medians
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::alloc::{count, measure};
    use std::hint::black_box;

    #[test]
    fn a_rows_are_rendered_with_quoting() {
        assert_eq!(csv_row(&[]), "");
        assert_eq!(csv_row(&["a"]), "a");
        assert_eq!(csv_row(&["a", "", "c"]), "a,,c");
        assert_eq!(csv_row(&["x,y", "z"]), "\"x,y\",z");
        assert_eq!(csv_row(&["say \"hi\""]), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_row(&["two\nlines", "é"]), "\"two\nlines\",é");
        assert_eq!(csv_row(&["\"", "\"\""]), "\"\"\"\",\"\"\"\"\"\"");
    }

    #[test]
    fn a_one_allocation_of_the_exact_size() {
        let fields = ["id", "name, first", "say \"hi\"", "", "plain text here"];
        let (row, usage) = measure(|| csv_row(&fields));
        assert_eq!(
            row,
            "id,\"name, first\",\"say \"\"hi\"\"\",,plain text here"
        );
        assert!(
            usage.allocations <= 1,
            "csv_row made {} allocations; compute the length first and allocate once",
            usage.allocations
        );
        assert!(
            usage.bytes <= row.len(),
            "csv_row asked the allocator for {} bytes to build a row of {} bytes; ask for \
             exactly the length",
            usage.bytes,
            row.len()
        );
        let (_, allocations) = count(|| csv_row(&[]));
        assert_eq!(allocations, 0, "an empty row needs no heap memory at all");
    }

    #[test]
    fn b_rows_are_appended() {
        let mut out = String::from("header\n");
        csv_rows_into(&[vec!["a", "b"], vec![], vec!["x,y"]], &mut out);
        assert_eq!(out, "header\na,b\n\n\"x,y\"\n");
        csv_rows_into(&[], &mut out);
        assert_eq!(out, "header\na,b\n\n\"x,y\"\n");
    }

    #[test]
    fn b_a_large_enough_buffer_is_reused() {
        let rows: Vec<Vec<&str>> = (0..200)
            .map(|i| vec!["alpha", if i % 3 == 0 { "b,c" } else { "beta" }, "\"q\""])
            .collect();
        let mut out = String::with_capacity(8 * 1024);
        let buffer = out.as_ptr();
        let (_, allocations) = count(|| csv_rows_into(&rows, &mut out));
        assert_eq!(
            allocations, 0,
            "csv_rows_into made {allocations} allocation(s) although `out` had room for \
             everything; write each row straight into `out`"
        );
        assert_eq!(
            out.as_ptr(),
            buffer,
            "`out` must still use the caller's buffer, not a new one"
        );
        assert_eq!(out.lines().count(), 200);
        assert!(out.starts_with("alpha,\"b,c\",\"\"\"q\"\"\"\nalpha,beta,"));
    }

    #[test]
    fn b_a_small_buffer_grows_once() {
        let rows: Vec<Vec<&str>> = (0..500).map(|_| vec!["some", "fields", "here"]).collect();
        let mut out = String::new();
        let (_, allocations) = count(|| csv_rows_into(&rows, &mut out));
        assert!(
            allocations <= 1,
            "csv_rows_into grew `out` {allocations} times; reserve the whole size before writing"
        );
        assert_eq!(out.len(), 500 * "some,fields,here\n".len());
    }

    #[test]
    fn c_runs_of_spaces_become_one_space() {
        assert_eq!(squeeze_spaces(""), "");
        assert_eq!(squeeze_spaces("a b c"), "a b c");
        assert_eq!(squeeze_spaces("a  b"), "a b");
        assert_eq!(squeeze_spaces("  a     b  "), " a b ");
        assert_eq!(squeeze_spaces("é  ü   ß"), "é ü ß");
        assert_eq!(squeeze_spaces("tabs\t\tstay  here"), "tabs\t\tstay here");
    }

    #[test]
    fn c_borrows_when_nothing_changes() {
        let input = "already clean, one space each";
        let (result, allocations) = count(|| squeeze_spaces(input));
        assert!(
            matches!(result, Cow::Borrowed(_)),
            "an input without a run of spaces must come back as Cow::Borrowed"
        );
        assert_eq!(
            result.as_ptr(),
            input.as_ptr(),
            "the borrowed result must be the input itself, not a copy"
        );
        assert_eq!(
            allocations, 0,
            "squeeze_spaces made {allocations} allocation(s) for an input that needs no change"
        );
    }

    #[test]
    fn c_allocates_once_when_something_changes() {
        let input = "a  b".repeat(500);
        let (result, allocations) = count(|| squeeze_spaces(&input));
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result.len(), 500 * 3);
        assert!(
            allocations <= 1,
            "squeeze_spaces made {allocations} allocations; the result is never longer than \
             the input, so one buffer of that size is enough"
        );
    }

    #[test]
    fn d_words_are_counted() {
        let counts = tally(&["b", "a", "b", "c", "b", "a"]);
        assert_eq!(counts.len(), 3);
        assert_eq!(counts["a"], 2);
        assert_eq!(counts["b"], 3);
        assert_eq!(counts["c"], 1);
        assert!(tally(&[]).is_empty());
    }

    #[test]
    fn d_a_known_word_costs_no_allocation() {
        let distinct = [
            "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
            "lambda", "mu", "nu", "xi", "omicron", "pi",
        ];
        let words: Vec<&str> = (0..10_000).map(|i| distinct[(i * 7) % 16]).collect();
        let (counts, allocations) = count(|| tally(&words));
        assert_eq!(counts.len(), 16);
        assert_eq!(counts.values().sum::<usize>(), 10_000);
        assert!(
            allocations <= 16 + 8,
            "tally made {allocations} allocations for 10,000 words of which 16 are distinct; \
             the budget is 16 keys plus 8 for the map. Look a word up with the `&str` you \
             have before you build a `String` for it"
        );
    }

    #[test]
    fn e_medians_are_correct_and_rows_stay_untouched() {
        let rows = vec![vec![5, 1, 4], vec![], vec![9], vec![2, 8, 4, 6], vec![3, 3]];
        assert_eq!(row_medians(&rows), [4, 0, 9, 6, 3]);
        assert_eq!(rows[0], [5, 1, 4], "the rows must not be sorted in place");
        assert_eq!(row_medians(&[]), []);
    }

    #[test]
    fn e_all_rows_share_one_scratch_buffer() {
        // Row lengths go up and down, so a buffer sized for the first row is
        // too small later.
        let mut rows: Vec<Vec<u32>> = (0..1_000u32)
            .map(|i| {
                (0..(i * 37 % 200))
                    .map(|j| (i * 31 + j * 17) % 1_000)
                    .collect()
            })
            .collect();
        let (medians, allocations) = count(|| row_medians(&rows));
        assert_eq!(medians.len(), 1_000);
        assert!(
            allocations <= 2,
            "row_medians made {allocations} allocations for 1,000 rows; the budget is 2: the \
             result and one scratch buffer, sized for the longest row before the loop"
        );

        // One long row among short ones: the numbers 0 to 19,999 in a
        // scrambled order (7,919 is a prime that does not divide 20,000).
        rows[500] = (0..20_000u32).map(|j| j * 7_919 % 20_000).collect();
        let (medians, allocations) = count(|| row_medians(&rows));
        assert_eq!(medians[500], 10_000, "the median of the long row");
        assert!(
            allocations <= 2,
            "row_medians made {allocations} allocations when one row has 20,000 values; the \
             budget is still 2. If the short rows passed, the extra allocation is the sort: \
             `sort` is stable and asks for scratch memory of its own for a long slice. \
             `sort_unstable` sorts in place"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// `black_box` only keeps the optimizer from removing the code; see lesson 01.
    #[test]
    fn p_predictions() {
        // P1. Allocations made by each of the three closures.
        let (_, filled) = count(|| {
            let mut v: Vec<u32> = Vec::with_capacity(100);
            for i in 0..100 {
                v.push(i);
            }
            black_box(v)
        });
        let (_, unit) = count(|| black_box(Box::new(())));
        let (_, empty) = count(|| {
            let v: Vec<String> = black_box(Vec::new());
            let s = black_box(String::new());
            (v, s)
        });
        let p1: (usize, usize, usize) = (1, 0, 0);
        assert_eq!(
            p1,
            (filled, unit, empty),
            "P1: allocations for (100 pushes into a Vec with capacity 100, Box::new(()), an empty Vec and String) (left: your prediction, right: actual)"
        );

        // P2. A buffer is emptied in two different ways and then refilled.
        let mut cleared = String::with_capacity(32);
        cleared.push_str("first");
        cleared.clear();
        let (_, after_clear) = count(|| cleared.push_str(black_box("second")));
        let mut taken = String::with_capacity(32);
        taken.push_str("first");
        let old = std::mem::take(&mut taken);
        let (_, after_take) = count(|| taken.push_str(black_box("second")));
        drop(old);
        let p2: (usize, usize) = (0, 1);
        assert_eq!(
            p2,
            (after_clear, after_take),
            "P2: allocations of the refill after (clear, mem::take) (left: your prediction, right: actual)"
        );

        // P3. Turning a `Cow` into an owned `String`.
        let borrowed: Cow<str> = Cow::Borrowed("some text");
        let owned: Cow<str> = Cow::Owned(String::from("some text"));
        let (_, from_borrowed) = count(|| black_box(borrowed.into_owned()));
        let (_, from_owned) = count(|| black_box(owned.into_owned()));
        let p3: (usize, usize) = (1, 0);
        assert_eq!(
            p3,
            (from_borrowed, from_owned),
            "P3: allocations of into_owned on (Cow::Borrowed, Cow::Owned) (left: your prediction, right: actual)"
        );

        // P4. Bytes that the allocator is asked for.
        let (_, usage) = measure(|| black_box(Vec::<u64>::with_capacity(10)));
        let p4: usize = 80;
        assert_eq!(
            p4, usage.bytes,
            "P4: bytes requested by Vec::<u64>::with_capacity(10) (left: your prediction, right: actual)"
        );
    }
}
