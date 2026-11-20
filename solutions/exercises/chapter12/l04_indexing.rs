//! LESSON 04 — Index and IndexMut
//! Run: cargo test --test chapter12 l04_
//! Read first: "Lesson 04" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Why does Index::index return a reference? Is
//! an indexing panic an appropriate replacement for a checked get API?
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Which trait enables reads and which enables writes? Why is returning
//! a reference to a temporary incorrect?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch12_scratch.rs):
//! Attempt to implement Index with Output=str and return an allocated String. Then
//! implement Index<Range<usize>> for a wrapper over Vec<i32> and observe the unsized
//! Output=[i32].
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. A lesson is
//! complete only when its tests, predictions, and explanation are done.

use std::ops::{Index, IndexMut};
#[derive(Debug)]
struct Grid([i32; 6]);
impl Grid {
    /// Position of (row, col) in the flat array. Both `index` and `index_mut`
    /// use it, so reads and writes can never disagree about the mapping.
    fn offset(row: usize, col: usize) -> usize {
        // Check each coordinate by itself: (0, 3) maps to offset 3, which is
        // inside the array but belongs to row 1.
        assert!(row < 2 && col < 3, "grid coordinate out of bounds");
        row * 3 + col
    }
}
impl Index<(usize, usize)> for Grid {
    type Output = i32;
    /// A. The grid has 2 rows of 3 columns. The array stores row 0 first, then
    /// row 1 (row-major order). Return the cell at (row, col). Panic with
    /// "grid coordinate out of bounds" when `row` or `col` is outside the grid.
    fn index(&self, (row, col): (usize, usize)) -> &i32 {
        &self.0[Grid::offset(row, col)]
    }
}
impl IndexMut<(usize, usize)> for Grid {
    /// B. Same coordinate contract, permitting in-place mutation.
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut i32 {
        &mut self.0[Grid::offset(row, col)]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::{panic_message, run_op};
    use std::collections::VecDeque;

    // Indexing runs through `run_op` and `panic_message` (see mod.rs). They fail
    // with a message if `index` or `index_mut` calls itself forever, for example
    // by writing `&mut self[(row, col)]` inside `index_mut`.

    /// Checks that `access` panics with the documented message.
    #[track_caller]
    fn assert_rejected(what: &str, access: impl FnOnce() + Send + 'static) {
        let message = panic_message(what, access);
        assert!(
            message
                .as_deref()
                .is_some_and(|text| text.contains("grid coordinate out of bounds")),
            "{what} must panic with \"grid coordinate out of bounds\". \
             Its panic message was: {message:?} (None means it did not panic)"
        );
    }

    #[test]
    fn a_index_maps_rows_and_columns() {
        let cells = run_op("reading Grid[(row, col)]", || {
            let g = Grid([0, 1, 2, 3, 4, 5]);
            [g[(0, 2)], g[(1, 0)], g[(1, 2)]]
        });
        assert_eq!(cells, [2, 3, 5], "cells (0, 2), (1, 0) and (1, 2)");
    }
    #[test]
    fn b_mutable_index_changes_one_element() {
        let cells = run_op("writing Grid[(row, col)]", || {
            let mut g = Grid([0; 6]);
            g[(1, 1)] = 9;
            g.0
        });
        assert_eq!(cells, [0, 0, 0, 0, 9, 0]);
    }
    #[test]
    fn a_rejects_invalid_column() {
        assert_rejected("reading Grid[(0, 3)]", || {
            let _ = Grid([0; 6])[(0, 3)];
        });
    }
    #[test]
    fn a_rejects_invalid_row() {
        assert_rejected("reading Grid[(2, 0)]", || {
            let _ = Grid([0; 6])[(2, 0)];
        });
    }
    #[test]
    fn b_rejects_invalid_row() {
        assert_rejected("writing Grid[(usize::MAX, 0)]", || {
            Grid([0; 6])[(usize::MAX, 0)] = 1;
        });
    }

    #[test]
    fn b_rejects_invalid_column_even_when_the_flat_index_fits() {
        // (0, 3) would be element 3 in a flat array: the row-major formula alone
        // does not catch it.
        assert_rejected("writing Grid[(0, 3)]", || {
            Grid([0; 6])[(0, 3)] = 1;
        });
    }

    #[test]
    fn b_writes_reach_the_same_element_reads_use() {
        let (cells, middle) = run_op("writing then reading Grid[(row, col)]", || {
            let mut g = Grid([0; 6]);
            for row in 0..2 {
                for col in 0..3 {
                    g[(row, col)] = (row * 10 + col) as i32;
                }
            }
            (g.0, g[(1, 1)])
        });
        assert_eq!(cells, [0, 1, 2, 10, 11, 12]);
        assert_eq!(middle, 11, "a read must find the value a write stored");
    }

    #[test]
    fn a_index_returns_a_reference_into_the_grid() {
        let points_into_grid = run_op("borrowing Grid[(row, col)]", || {
            let g = Grid([0, 1, 2, 3, 4, 5]);
            std::ptr::eq(&g[(1, 2)], &g.0[5])
        });
        assert!(
            points_into_grid,
            "&grid[(1, 2)] must point at the element stored in the grid, not at a copy"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Index with ranges: a slice of a slice, and a byte range of a str.
        let p1: (i32, &str) = (20, "é");
        assert_eq!(
            p1,
            ([10, 20, 30][1..][0], &"héllo"[1..3]),
            "P1: Index<Range<usize>>"
        );

        // P2. VecDeque implements Index too. After push_front, what is at 0 and 2?
        let mut d = VecDeque::from([1, 2]);
        d.push_front(0);
        let p2: (i32, i32) = (0, 2);
        assert_eq!(p2, (d[0], d[2]), "P2: VecDeque positions after push_front");

        // P3. `v[i]` means `*v.index(i)`. Does `&v[1]` point into the array itself?
        let v = [String::from("a"), String::from("b")];
        let p3: bool = true;
        assert_eq!(
            p3,
            std::ptr::eq(&v[1], v.as_ptr().wrapping_add(1)),
            "P3: does &v[1] point into the array?"
        );
    }
}
