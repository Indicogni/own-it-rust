//! LESSON 08 — Disjoint mutable borrows
//! Run: cargo test --test chapter05 l08_
//! Read first: CONCEPTS.md, section "Lesson 08 · Disjoint mutable borrows".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Will borrowing &mut values[0] and &mut values[1] separately compile? How can split_at_mut hand out two &mut references safely?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain what split_at_mut guarantees, where that guarantee comes from, and why the caller needs no unsafe code.
//! COMPILER / TRANSFER LAB (optional; tests do not check it): Try the two indexing borrows, then split_at_mut. Contrast borrowing two distinct struct fields, which the compiler can already distinguish.
//! Use examples/ch05_scratch.rs; repair intentional failures afterward.

/// A. Add delta to every item before index mid (`values[..mid]`) and subtract
/// it from every item from mid on (`values[mid..]`). Return true. If mid > len,
/// return false without mutation. Inputs and delta are within -100..100.
fn adjust_halves(values: &mut [i32], mid: usize, delta: i32) -> bool {
    // The checked variant returns None instead of panicking when mid > len.
    let Some((left, right)) = values.split_at_mut_checked(mid) else {
        return false;
    };
    for value in left {
        *value += delta;
    }
    for value in right {
        *value -= delta;
    }
    true
}
/// B. Lend out two DIFFERENT elements at the same time: return
/// `Some((&mut values[a], &mut values[b]))`, in that order, whether a < b or
/// a > b. Return None when a == b (two `&mut` to one element must never exist)
/// or when an index is out of range. Never panic and never clone.
/// Build it from `split_at_mut`. The standard library also offers
/// `get_disjoint_mut` for exactly this job, and the tests cannot tell the two
/// apart; try it only after your own version passes.
fn pair_mut(values: &mut [String], a: usize, b: usize) -> Option<(&mut String, &mut String)> {
    if a == b || a >= values.len() || b >= values.len() {
        return None;
    }
    // Split at the LARGER index: the smaller one is then in the left half and
    // the larger one is element 0 of the right half.
    if a < b {
        let (left, right) = values.split_at_mut(b);
        Some((&mut left[a], &mut right[0]))
    } else {
        let (left, right) = values.split_at_mut(a);
        Some((&mut right[0], &mut left[b]))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_halves_are_nonoverlapping() {
        let mut a = [1, 2, 3];
        assert!(adjust_halves(&mut a, 1, 2));
        assert_eq!(a, [3, 0, 1]);
        assert!(!adjust_halves(&mut a, 4, 9));
        assert_eq!(a, [3, 0, 1]);
        assert!(adjust_halves(&mut a, 3, 1));
        assert_eq!(a, [4, 1, 2]);
        assert!(adjust_halves(&mut a, 0, 1));
        assert_eq!(a, [3, 0, 1]);
        assert!(adjust_halves(&mut [], 0, 1));
    }
    #[test]
    fn b_pair_handles_both_index_orders() {
        let mut v: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        let (x, y) = pair_mut(&mut v, 2, 0).unwrap();
        assert_eq!(
            (x.as_str(), y.as_str()),
            ("c", "a"),
            "the first result is values[a] and the second is values[b], even when a > b"
        );
        std::mem::swap(x, y);
        y.push('!');
        assert_eq!(v, ["c!", "b", "a"], "writes must land in the slice itself");
        let (x, y) = pair_mut(&mut v, 1, 2).unwrap();
        x.push_str(y);
        assert_eq!(v, ["c!", "ba", "a"]);
    }

    #[test]
    fn b_pair_refuses_equal_or_invalid_indices() {
        let mut v: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        assert!(
            pair_mut(&mut v, 1, 1).is_none(),
            "a == b would hand out two &mut to one element"
        );
        assert!(pair_mut(&mut v, 1, 3).is_none(), "b is out of range");
        assert!(pair_mut(&mut v, 3, 1).is_none(), "a is out of range");
        assert!(pair_mut(&mut v, 3, 3).is_none());
        assert!(pair_mut(&mut v, 0, 2).is_some());
        assert!(pair_mut(&mut [], 0, 0).is_none());
        assert_eq!(v, ["a", "b", "c"]);
    }

    #[test]
    fn b_pair_points_into_the_slice_without_cloning() {
        let mut values: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        let first: *const String = &values[0];
        let last: *const String = &values[2];
        let (pair, allocations) = count(|| {
            pair_mut(&mut values, 2, 0).map(|(x, y)| (x as *const String, y as *const String))
        });
        assert_eq!(
            pair,
            Some((last, first)),
            "the results must be the two elements themselves, in the order asked"
        );
        assert_eq!(allocations, 0, "lend the Strings out; don't clone them");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Which half does the element at `mid` land in?
        let mut a = [1, 2, 3, 4, 5];
        let (left, right) = a.split_at_mut(2);
        let p1: (usize, i32) = (2, 3);
        assert_eq!(p1, (left.len(), right[0]), "P1: split_at_mut(2)");

        // P2. One write per half, each reading the other half.
        left[0] += right[0];
        right[2] = left[1];
        let p2: [i32; 5] = [4, 2, 3, 4, 2];
        assert_eq!(p2, a, "P2: disjoint halves");

        // P3. Two mutable borrows of different tuple fields.
        let mut pair = (1, 2);
        let x = &mut pair.0;
        let y = &mut pair.1;
        *x += *y;
        *y += *x;
        let p3: (i32, i32) = (3, 5);
        assert_eq!(p3, pair, "P3: fields are disjoint places");

        // P4. chunks_mut yields non-overlapping mutable slices.
        let mut data = [1, 2, 3, 4, 5];
        for chunk in data.chunks_mut(2) {
            chunk[0] *= 10;
        }
        let p4: [i32; 5] = [10, 2, 30, 4, 50];
        assert_eq!(p4, data, "P4: chunks_mut");

        // P5. `get_disjoint_mut` asks for several elements at once and checks
        // the indices at run time. Is each of these three requests refused?
        let mut v = [1, 2, 3];
        let p5: (bool, bool, bool) = (true, true, false);
        assert_eq!(
            p5,
            (
                v.get_disjoint_mut([0, 0]).is_err(),
                v.get_disjoint_mut([0, 3]).is_err(),
                v.get_disjoint_mut([2, 0]).is_err()
            ),
            "P5: get_disjoint_mut refuses equal or out-of-range indices"
        );
    }
}
