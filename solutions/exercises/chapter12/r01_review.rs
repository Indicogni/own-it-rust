//! REVIEW 01 — Spaced retrieval: Chapters 5, 10, and 11 inside Chapter 12
//! Run: cargo test --test chapter12 r01_
//!
//! Do this WITHOUT reopening Chapters 5, 10, or 11 or their hints. Struggling
//! to recall is what makes the knowledge stick; look things up only after a
//! real attempt, and note in NOTES.md anything you had to look up.
//!
//! Each task mixes an earlier idea (generic bounds, enum patterns, returning
//! references) with this chapter's operator traits.

use std::ops::{Add, Index, Mul};

/// A. (Ch11 generic bounds + Ch12 arithmetic traits) Return the dot product
/// (sum of pairwise products) of two equally long slices, starting from
/// `T::default()`. Return None when the lengths differ.
fn dot<T>(a: &[T], b: &[T]) -> Option<T>
where
    T: Copy + Default + Add<Output = T> + Mul<Output = T>,
{
    if a.len() != b.len() {
        return None;
    }
    Some(
        a.iter()
            .zip(b)
            .fold(T::default(), |sum, (&x, &y)| sum + x * y),
    )
}

#[derive(Debug, Clone, Copy)]
enum Money {
    Cents(u32),
    Dollars(u32),
}

impl Money {
    /// The amount in cents. `u64` holds `u32::MAX * 100` with room to spare.
    fn cents(self) -> u64 {
        match self {
            Money::Cents(cents) => u64::from(cents),
            Money::Dollars(dollars) => u64::from(dollars) * 100,
        }
    }
}

impl PartialEq for Money {
    /// B. (Ch10 enum patterns + Ch12 equivalence) Two amounts are equal when they
    /// are worth the same number of cents, whatever the variants:
    /// Dollars(1) == Cents(100). Never overflow, even for Dollars(u32::MAX).
    fn eq(&self, other: &Self) -> bool {
        self.cents() == other.cents()
    }
}

impl Eq for Money {}

struct Table {
    rows: Vec<(String, u32)>,
}

impl Table {
    /// C1. (Ch5 references) Borrow the value stored for `name`; None if absent.
    /// If a name appears more than once, the first row wins.
    fn get(&self, name: &str) -> Option<&u32> {
        self.rows
            .iter()
            .find(|(row_name, _)| row_name == name)
            .map(|(_, value)| value)
    }
}

impl Index<&str> for Table {
    type Output = u32;
    /// C2. (Ch12 Index) `table["name"]` reads like `get`, but panics with
    /// "no row named NAME" when the name is absent. Reuse `get`.
    fn index(&self, name: &str) -> &u32 {
        self.get(name)
            .unwrap_or_else(|| panic!("no row named {name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guard::{panic_message, run_op};

    #[test]
    fn a_dot_product_is_generic() {
        assert_eq!(dot(&[1, 2, 3], &[4, -5, 6]), Some(12));
        assert_eq!(dot(&[0.5, 2.0], &[4.0, 0.25]), Some(2.5));
        assert_eq!(dot::<u8>(&[], &[]), Some(0));
        assert_eq!(dot(&[1, 2], &[1]), None);
    }

    // `==` and indexing run through `run_op` (see mod.rs), which fails with a
    // message if `eq` or `index` calls itself forever.
    #[test]
    fn b_money_compares_across_variants() {
        let (dollar, same_cents, less, max, wrapped, large) = run_op("Money == Money", || {
            (
                Money::Dollars(1) == Money::Cents(100),
                Money::Cents(250) == Money::Cents(250),
                Money::Cents(250) == Money::Dollars(2),
                Money::Dollars(u32::MAX) == Money::Cents(u32::MAX),
                // 42_949_673 * 100 wraps to 4 in u32 arithmetic.
                Money::Dollars(42_949_673) == Money::Cents(4),
                Money::Dollars(42_949_672) == Money::Cents(4_294_967_200),
            )
        });
        assert!(dollar, "Dollars(1) is worth Cents(100)");
        assert!(same_cents, "Cents(250) equals Cents(250)");
        assert!(!less, "Cents(250) is more than Dollars(2)");
        assert!(!max, "Dollars(u32::MAX) is far more than Cents(u32::MAX)");
        assert!(
            !wrapped,
            "Dollars(42_949_673) is not Cents(4): the conversion must not wrap around"
        );
        assert!(large, "Dollars(42_949_672) is worth Cents(4_294_967_200)");
    }

    fn table() -> Table {
        Table {
            rows: vec![("ada".into(), 36), ("bob".into(), 7), ("ada".into(), 99)],
        }
    }

    #[test]
    fn c_table_borrows_and_indexes() {
        let t = table();
        assert_eq!(t.get("bob"), Some(&7));
        assert_eq!(t.get("ada"), Some(&36), "the first row with that name wins");
        assert_eq!(t.get("Bob"), None, "names are compared exactly");
        assert!(
            std::ptr::eq(t.get("bob").unwrap(), &t.rows[1].1),
            "get must return a reference to the number stored in the row, not to a copy"
        );
        let (points_into_row, sum) = run_op("Table[name]", || {
            let t = table();
            (std::ptr::eq(&t["bob"], &t.rows[1].1), t["ada"] + t["bob"])
        });
        assert!(
            points_into_row,
            "table[name] must refer to the number stored in the row, not to a copy"
        );
        assert_eq!(sum, 43);
    }

    #[test]
    fn c_index_panics_for_a_missing_name() {
        let message = panic_message("Table[\"zed\"] on an empty table", || {
            let t = Table { rows: vec![] };
            let _ = t["zed"];
        });
        assert!(
            message
                .as_deref()
                .is_some_and(|text| text.contains("no row named zed")),
            "indexing a missing name must panic with \"no row named zed\". \
             Its panic message was: {message:?} (None means it did not panic)"
        );
    }

    /// P. Chapters 3, 7, 10, and 11 recall: predict each value BEFORE running.
    #[test]
    fn p_predictions() {
        // P1 (Ch11). Size of a boxed trait object, in machine words.
        let p1: usize = 2;
        assert_eq!(
            p1 * size_of::<usize>(),
            size_of::<Box<dyn Fn()>>(),
            "P1: size of Box<dyn Fn()> in words"
        );

        // P2 (Ch10). Slice patterns.
        let v = [1, 2, 3, 4];
        let [first, .., last] = v;
        let p2: i32 = 14;
        assert_eq!(p2, first * 10 + last, "P2: [first, .., last]");

        // P3 (Ch7). Recovering from a parse failure.
        let p3: u8 = 7;
        assert_eq!(p3, "300".parse::<u8>().unwrap_or(7), "P3: unwrap_or");

        // P4 (Ch3). `as` casts between integer types.
        let p4: (i8, bool) = (-56, true);
        assert_eq!(
            p4,
            (200_u8 as i8, -1_i32 as u32 == u32::MAX),
            "P4: `as` between integer types"
        );
    }
}
