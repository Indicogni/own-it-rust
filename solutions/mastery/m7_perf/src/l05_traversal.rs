//! LESSON 05 — Traversal order, cache misses, and bounds checks
//! Read first: CONCEPTS.md, "Lesson 05". Start after: lesson 04; chapters 15
//! (iterators) and 16 (slices).
//! Run: cargo test --manifest-path mastery/m7_perf/Cargo.toml l05_
//! Bench: cargo bench --manifest-path mastery/m7_perf/Cargo.toml --bench l05_traversal
//! Lab: LABS.md, "Lab 5" looks for the bounds checks in the compiler's output.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: two loops add up the same
//! 4,096 by 4,096 grid of `u32`. One goes row by row, the other column by
//! column. They execute the same additions. Why can one be several times
//! slower than the other?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! From this lesson on you get the signatures and decide the structure of the
//! solution yourself. The tests judge that structure with two instruments that
//! are supplied below. Read them first; they are short.
//! - `Lru` is a model of a small cache. `Traced` is a block of memory that
//!   reports every read and write to such a model. A budget on the misses of
//!   the model is a budget on the order in which your code walks over memory.
//! - `Checked` is a slice that counts the bounds checks your code asks for.
//!
//! EXPLAIN afterward: Why does the tiled transpose miss less often than the
//! plain one, although both read and write every element exactly once? What
//! does the `Lru` model leave out that a real cache has? Why is a check before
//! the loop better than a check in every iteration, and why is no check at all
//! (an iterator) better still?
//! JUDGMENT: Walk memory in the order in which it is laid out; that single
//! habit removes most cache problems. Tile a loop only when a profile shows
//! cache misses and the data does not fit in the cache; tiling makes code
//! harder to read. For bounds checks, write iterator code first: it has no
//! checks to remove. Where you must index, check the range once before the
//! loop by taking a sub-slice. Reach for `get_unchecked` last, with a
//! benchmark that shows the gain and a SAFETY comment that proves the index
//! is in range.

use std::cell::{Cell, RefCell};
use std::ops::Range;

// ---------------------------------------------------------------------------
// Instrument 1: a cache model. Supplied; read it, do not edit it.
// ---------------------------------------------------------------------------

/// The size of one cache line of the model in bytes: 64, as on most x86-64
/// and ARM processors (Apple silicon uses 128). A cache loads and evicts
/// whole lines.
pub const LINE: usize = 64;

/// A model of a small cache: it holds the `capacity` most recently used
/// lines. Using a line that is not in the cache is a *miss*; the line is
/// loaded, and if the cache is full, the line that was used longest ago is
/// evicted.
#[derive(Debug)]
pub struct Lru {
    /// `(line number, time of last use)` of every line in the cache.
    lines: Vec<(usize, u64)>,
    capacity: usize,
    now: u64,
    pub hits: u64,
    pub misses: u64,
}

impl Lru {
    pub fn new(capacity: usize) -> Lru {
        assert!(capacity > 0, "a cache holds at least one line");
        Lru {
            lines: Vec::with_capacity(capacity),
            capacity,
            now: 0,
            hits: 0,
            misses: 0,
        }
    }

    /// Records one access to the byte at `address`.
    pub fn touch(&mut self, address: usize) {
        let line = address / LINE;
        self.now += 1;
        if let Some(entry) = self.lines.iter_mut().find(|entry| entry.0 == line) {
            entry.1 = self.now;
            self.hits += 1;
            return;
        }
        self.misses += 1;
        if self.lines.len() == self.capacity {
            let oldest = (0..self.lines.len())
                .min_by_key(|&at| self.lines[at].1)
                .expect("the cache is not empty");
            self.lines.swap_remove(oldest);
        }
        self.lines.push((line, self.now));
    }
}

/// A block of `u32` cells that code reads through an index.
pub trait Cells {
    fn len(&self) -> usize;
    fn read(&self, index: usize) -> u32;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A block of `u32` cells that code can also write.
pub trait CellsMut: Cells {
    fn write(&mut self, index: usize, value: u32);
}

impl Cells for [u32] {
    fn len(&self) -> usize {
        <[u32]>::len(self)
    }
    fn read(&self, index: usize) -> u32 {
        self[index]
    }
}

impl CellsMut for [u32] {
    fn write(&mut self, index: usize, value: u32) {
        self[index] = value;
    }
}

/// Cells that report every access to a cache model. Cell `i` lies at the
/// address `base + 4 * i`.
pub struct Traced<'c> {
    cells: Vec<u32>,
    base: usize,
    cache: &'c RefCell<Lru>,
}

impl<'c> Traced<'c> {
    pub fn new(cells: Vec<u32>, base: usize, cache: &'c RefCell<Lru>) -> Traced<'c> {
        Traced { cells, base, cache }
    }

    /// The cells, without recording an access.
    pub fn into_cells(self) -> Vec<u32> {
        self.cells
    }
}

impl Cells for Traced<'_> {
    fn len(&self) -> usize {
        self.cells.len()
    }
    fn read(&self, index: usize) -> u32 {
        self.cache.borrow_mut().touch(self.base + 4 * index);
        self.cells[index]
    }
}

impl CellsMut for Traced<'_> {
    fn write(&mut self, index: usize, value: u32) {
        self.cache.borrow_mut().touch(self.base + 4 * index);
        self.cells[index] = value;
    }
}

// ---------------------------------------------------------------------------
// Instrument 2: a slice that counts bounds checks. Supplied; do not edit it.
// ---------------------------------------------------------------------------

/// A slice that counts how many bounds checks the code that uses it asks for.
///
/// The count is a property of the source code. The optimizer removes a check
/// when it can prove that the index is in range, and keeps it otherwise. Code
/// that asks for no check does not depend on such a proof.
pub struct Checked<'a, T> {
    items: &'a [T],
    checks: &'a Cell<u64>,
}

impl<'a, T> Checked<'a, T> {
    pub fn new(items: &'a [T], checks: &'a Cell<u64>) -> Checked<'a, T> {
        Checked { items, checks }
    }

    /// No check: the length is stored next to the pointer.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// One element, like `items[index]`: **one bounds check**. Panics when
    /// `index` is out of range.
    pub fn at(&self, index: usize) -> &'a T {
        self.checks.set(self.checks.get() + 1);
        &self.items[index]
    }

    /// A part of the slice, like `&items[range]`: **one bounds check** for
    /// the whole range. Panics when the range does not fit.
    pub fn slice(&self, range: Range<usize>) -> Checked<'a, T> {
        self.checks.set(self.checks.get() + 1);
        Checked {
            items: &self.items[range],
            checks: self.checks,
        }
    }

    /// All elements in order, like `items.iter()`: **no bounds check**. An
    /// iterator moves from the first element to the last one and stops, so it
    /// cannot leave the slice.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &'a T> + ExactSizeIterator + use<'a, T> {
        self.items.iter()
    }
}

// ---------------------------------------------------------------------------
// Your tasks.
// ---------------------------------------------------------------------------

/// A. `cells` holds a grid in row-major order: row 0 first, then row 1, and so
/// on; every row has `width` cells. Return the sum of every column.
///
/// `width` is at least 1 and divides the number of cells; panic otherwise.
///
/// Budget: with a cache model that is much smaller than the grid, every cache
/// line of the grid may miss only once.
pub fn column_sums<C: Cells + ?Sized>(cells: &C, width: usize) -> Vec<u64> {
    assert!(
        width > 0 && cells.len().is_multiple_of(width),
        "the grid must consist of whole rows of `width` cells"
    );
    // The sums are few and stay in the cache. The grid is walked once, in
    // the order in which it lies in memory.
    let mut sums = vec![0u64; width];
    for index in 0..cells.len() {
        sums[index % width] += u64::from(cells.read(index));
    }
    sums
}

/// The same result as `column_sums`, computed column by column. Supplied, for
/// the benchmark and for comparison in the tests.
pub fn column_sums_by_column<C: Cells + ?Sized>(cells: &C, width: usize) -> Vec<u64> {
    assert!(width > 0 && cells.len().is_multiple_of(width));
    let height = cells.len() / width;
    (0..width)
        .map(|column| {
            (0..height)
                .map(|row| u64::from(cells.read(row * width + column)))
                .sum()
        })
        .collect()
}

/// B. `src` and `dst` each hold a square grid of `n` rows and `n` columns in
/// row-major order. Write the transpose of `src` into `dst`: the cell in row
/// `r`, column `c` of `src` goes to row `c`, column `r` of `dst`. Read every
/// cell of `src` once and write every cell of `dst` once.
///
/// Panics when a grid does not have `n * n` cells.
///
/// Budget: with a cache model of 64 lines and `n = 256`, at most twice as many
/// misses as the two grids have cache lines. A loop that goes along the rows
/// of one grid goes down the columns of the other one, so plain row order is
/// not enough. Work on one small square block of both grids at a time.
pub fn transpose<S, D>(src: &S, dst: &mut D, n: usize)
where
    S: Cells + ?Sized,
    D: CellsMut + ?Sized,
{
    assert!(
        src.len() == n * n && dst.len() == n * n,
        "both grids must have n * n cells"
    );
    // A block of 16 by 16 cells is 16 cache lines in `src` (one per row,
    // 16 cells of 4 bytes) and 16 lines in `dst`: 32 lines stay in use until
    // the block is done, which fits a 64-line cache with room to spare.
    const BLOCK: usize = 16;
    for row_start in (0..n).step_by(BLOCK) {
        for column_start in (0..n).step_by(BLOCK) {
            for row in row_start..(row_start + BLOCK).min(n) {
                for column in column_start..(column_start + BLOCK).min(n) {
                    dst.write(column * n + row, src.read(row * n + column));
                }
            }
        }
    }
}

/// C. The dot product of two sequences: the sum of `a[i] * b[i]` over the
/// positions that both have. The tests use small numbers; you need not think
/// about overflow.
///
/// Budget: no bounds check.
pub fn dot(a: &Checked<'_, i64>, b: &Checked<'_, i64>) -> i64 {
    // `zip` stops at the end of the shorter side, so no index exists that
    // could be out of range.
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// D. Interleave the first `n` elements of `left` and `right`:
/// `left[0], right[0], left[1], right[1], ...`.
///
/// Panics when `left` or `right` has fewer than `n` elements. A caller that
/// passes a wrong `n` has a bug, and it must not get a shorter result back
/// without notice.
///
/// Budget: at most two bounds checks in total, and one allocation.
pub fn interleave(left: &Checked<'_, u8>, right: &Checked<'_, u8>, n: usize) -> Vec<u8> {
    // One check per side, before the loop. Each one panics for a wrong `n`,
    // and after them both sides are known to have exactly `n` elements.
    let left = left.slice(0..n);
    let right = right.slice(0..n);
    let mut mixed = Vec::with_capacity(2 * n);
    for (&l, &r) in left.iter().zip(right.iter()) {
        mixed.push(l);
        mixed.push(r);
    }
    mixed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::alloc::count;

    /// Runs `f`, which must panic, and returns the panic message. A test
    /// that only says `#[should_panic]` would also pass while the task still
    /// holds `todo!()`.
    fn panic_message(what: &str, f: impl FnOnce()) -> String {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        let payload = outcome.expect_err(what);
        let message = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
            .unwrap_or_default();
        assert!(
            !message.contains("not yet implemented"),
            "implement the task first; the panic came from `todo!()`"
        );
        message
    }

    fn grid(rows: usize, columns: usize) -> Vec<u32> {
        (0..rows * columns)
            .map(|i| (i as u32).wrapping_mul(2_654_435_761) >> 20)
            .collect()
    }

    #[test]
    fn a_column_sums_are_correct() {
        let cells = [1, 2, 3, 10, 20, 30];
        assert_eq!(column_sums(&cells[..], 3), [11, 22, 33]);
        assert_eq!(column_sums(&cells[..], 6), [1, 2, 3, 10, 20, 30]);
        assert_eq!(column_sums(&cells[..], 1), [66]);
        assert_eq!(column_sums(&cells[..0], 4), [0, 0, 0, 0]);
        let big = grid(37, 53);
        assert_eq!(
            column_sums(&big[..], 53),
            column_sums_by_column(&big[..], 53)
        );
        let large = [u32::MAX; 8];
        assert_eq!(
            column_sums(&large[..], 2),
            [4 * u64::from(u32::MAX); 2],
            "the sums must not overflow a u32"
        );
    }

    #[test]
    fn a_a_ragged_grid_is_rejected() {
        panic_message(
            "five cells are not whole rows of two: this must panic",
            || {
                column_sums(&[1, 2, 3, 4, 5][..], 2);
            },
        );
        panic_message("a width of zero must panic", || {
            column_sums(&[1, 2, 3, 4][..], 0);
        });
    }

    #[test]
    fn a_every_line_of_the_grid_misses_once() {
        let (rows, columns) = (256, 256);
        let cache = RefCell::new(Lru::new(64));
        let traced = Traced::new(grid(rows, columns), 0, &cache);
        let sums = column_sums(&traced, columns);
        let misses = cache.borrow().misses;
        let cells = traced.into_cells();
        assert_eq!(sums, column_sums_by_column(&cells[..], columns));
        let lines = (rows * columns * 4 / LINE) as u64;
        assert!(
            misses <= lines,
            "column_sums caused {misses} modelled cache misses on a grid of {lines} cache \
             lines. Walk the cells in the order in which they lie in memory, so that all 16 \
             cells of a line are used while the line is in the cache"
        );
    }

    #[test]
    fn b_transpose_is_correct() {
        for n in [0, 1, 2, 5, 16, 33, 100] {
            let src = grid(n, n);
            let mut dst = vec![0u32; n * n];
            transpose(&src[..], &mut dst[..], n);
            for row in 0..n {
                for column in 0..n {
                    assert_eq!(
                        dst[column * n + row],
                        src[row * n + column],
                        "n = {n}, source row {row}, column {column}"
                    );
                }
            }
        }
    }

    #[test]
    fn b_a_wrong_size_is_rejected() {
        let src = grid(4, 4);
        let mut dst = [0u32; 15];
        panic_message("a destination of 15 cells for n = 4 must panic", || {
            transpose(&src[..], &mut dst[..], 4);
        });
        // The check must come before the first write.
        assert_eq!(
            dst, [0u32; 15],
            "nothing may be written before the sizes are checked"
        );
    }

    #[test]
    fn b_transpose_stays_within_the_miss_budget() {
        let n = 256;
        let cache = RefCell::new(Lru::new(64));
        let src = Traced::new(grid(n, n), 0, &cache);
        // The second grid lies somewhere else in memory.
        let mut dst = Traced::new(vec![0u32; n * n], 1 << 24, &cache);
        transpose(&src, &mut dst, n);
        let (hits, misses) = (cache.borrow().hits, cache.borrow().misses);
        assert_eq!(
            hits + misses,
            2 * (n * n) as u64,
            "read every source cell once and write every destination cell once"
        );
        let (src, dst) = (src.into_cells(), dst.into_cells());
        assert!((0..n * n).all(|i| dst[(i % n) * n + i / n] == src[i]));
        let lines = 2 * (n * n * 4 / LINE) as u64;
        assert!(
            misses <= 2 * lines,
            "transpose caused {misses} modelled cache misses; the two grids have {lines} \
             cache lines, and the budget is twice that. Going along the rows of one grid \
             means going down the columns of the other, and a column of 256 cells touches \
             256 different lines: more than the cache holds. Finish a small square block of \
             both grids before you move on. If you do that already, look at the block size: \
             a block that is narrower than a cache line (16 cells) loads every line several \
             times, and a block whose rows in both grids do not fit into 64 lines together \
             evicts its own lines"
        );
    }

    #[test]
    fn c_dot_products() {
        let checks = Cell::new(0);
        let a = [1i64, 2, 3, 4];
        let b = [10i64, 20, 30];
        let (a, b) = (Checked::new(&a, &checks), Checked::new(&b, &checks));
        assert_eq!(dot(&a, &b), 140, "the fourth element of `a` has no partner");
        assert_eq!(dot(&b, &a), 140);
        assert_eq!(dot(&a, &a.slice(0..0)), 0);
    }

    #[test]
    fn c_dot_asks_for_no_bounds_check() {
        let checks = Cell::new(0);
        let a: Vec<i64> = (0..1_000).collect();
        let b: Vec<i64> = (0..1_200).map(|i| i % 7).collect();
        let result = dot(&Checked::new(&a, &checks), &Checked::new(&b, &checks));
        assert_eq!(result, (0..1_000).map(|i| i * (i % 7)).sum::<i64>());
        assert_eq!(
            checks.get(),
            0,
            "dot asked for {} bounds checks on 1,000 pairs; walk both sequences with \
             iterators instead of indexing them",
            checks.get()
        );
    }

    #[test]
    fn d_interleaves_the_first_n() {
        let checks = Cell::new(0);
        let left = Checked::new(b"ace-", &checks);
        let right = Checked::new(b"bdf--", &checks);
        assert_eq!(interleave(&left, &right, 3), b"abcdef");
        assert_eq!(interleave(&left, &right, 4), b"abcdef--");
        assert_eq!(interleave(&left, &right, 0), b"");
    }

    #[test]
    fn d_too_large_an_n_panics() {
        let checks = Cell::new(0);
        let left = Checked::new(b"abc", &checks);
        let right = Checked::new(b"abcdef", &checks);
        // `left` has only three elements. Returning three pairs here would
        // hide the caller's bug.
        panic_message("n = 4 with a `left` of three elements must panic", || {
            interleave(&left, &right, 4);
        });
        panic_message("n = 4 with a `right` of three elements must panic", || {
            interleave(&right, &left, 4);
        });
    }

    #[test]
    fn d_checks_once_before_the_loop() {
        let checks = Cell::new(0);
        let left = vec![1u8; 5_000];
        let right = vec![2u8; 6_000];
        let (left, right) = (Checked::new(&left, &checks), Checked::new(&right, &checks));
        let (mixed, allocations) = count(|| interleave(&left, &right, 4_000));
        assert_eq!(mixed.len(), 8_000);
        assert!(mixed.chunks(2).all(|pair| pair == [1, 2]));
        assert!(
            checks.get() <= 2,
            "interleave asked for {} bounds checks for 4,000 pairs; the budget is 2. Take \
             the first `n` elements of each side as a sub-slice before the loop: that is one \
             check per side, and it still panics for a wrong `n`",
            checks.get()
        );
        assert!(
            allocations <= 1,
            "interleave made {allocations} allocations; the length of the result is known \
             before the loop"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Read 1,000 `u32` cells in order, starting at address 0, with a
        // cache model of 64 lines. How many misses?
        let cache = RefCell::new(Lru::new(64));
        let cells = Traced::new(vec![0; 1_000], 0, &cache);
        for index in 0..1_000 {
            cells.read(index);
        }
        let p1: u64 = 63;
        assert_eq!(
            p1,
            cache.borrow().misses,
            "P1: misses of one pass over 1,000 u32 cells (left: your prediction, right: actual)"
        );

        // P2. Read every cell of a block twice: one full pass, then a second
        // full pass. The cache model holds 64 lines. How many misses in total
        // for a block of 100 lines, and for a block of 50 lines?
        let two_passes = |lines: usize| {
            let cache = RefCell::new(Lru::new(64));
            let cells = Traced::new(vec![0; lines * LINE / 4], 0, &cache);
            for _pass in 0..2 {
                for index in 0..cells.len() {
                    cells.read(index);
                }
            }
            cache.borrow().misses
        };
        let p2: (u64, u64) = (200, 50);
        assert_eq!(
            p2,
            (two_passes(100), two_passes(50)),
            "P2: misses of two passes over (100 lines, 50 lines) (left: your prediction, right: actual)"
        );

        // P3. A grid of 64 rows and 64 columns of `u32`, and a cache model of
        // 16 lines. How many misses when the grid is read row by row, and how
        // many when it is read column by column?
        let read_grid = |by_rows: bool| {
            let cache = RefCell::new(Lru::new(16));
            let cells = Traced::new(vec![0; 64 * 64], 0, &cache);
            for outer in 0..64 {
                for inner in 0..64 {
                    let (row, column) = if by_rows {
                        (outer, inner)
                    } else {
                        (inner, outer)
                    };
                    cells.read(row * 64 + column);
                }
            }
            cache.borrow().misses
        };
        let p3: (u64, u64) = (256, 4096);
        assert_eq!(
            p3,
            (read_grid(true), read_grid(false)),
            "P3: misses when reading (row by row, column by column) (left: your prediction, right: actual)"
        );

        // P4. Bounds checks that two ways of adding up neighbours ask for.
        let values: Vec<u64> = (0..1_000).collect();
        let checks = Cell::new(0);
        let checked = Checked::new(&values, &checks);
        let mut by_index = 0;
        for i in 0..checked.len() - 1 {
            by_index += checked.at(i) + checked.at(i + 1);
        }
        let indexed = checks.replace(0);
        let by_zip: u64 = checked
            .iter()
            .zip(checked.iter().skip(1))
            .map(|(a, b)| a + b)
            .sum();
        let zipped = checks.get();
        assert_eq!(by_index, by_zip);
        let p4: (u64, u64) = (1998, 0);
        assert_eq!(
            p4,
            (indexed, zipped),
            "P4: bounds checks asked for by (the index loop, the zip) (left: your prediction, right: actual)"
        );
    }
}
