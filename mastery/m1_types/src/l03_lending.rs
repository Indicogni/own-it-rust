//! LESSON 03 — Generic associated types and a lending iterator
//! Run (in mastery/m1_types/): cargo test --no-fail-fast l03_
//! Read first: "Lesson 03" in CONCEPTS.md (one directory up).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `Iterator::next` returns `Option<Self::Item>`. Where could a lifetime
//!    that is tied to the `&mut self` of one call be written in that type?
//! 2. `[T]::windows` exists, and `[T]::windows_mut` does not. What would
//!    `collect()` on overlapping mutable windows give you?
//! 3. What does `where Self: 'a` on `type Item<'a>` say about `'a`?
//!
//! IMPLEMENT the tasks below. Tasks A and B have unit tests in this file.
//! Tasks C and D have no skeleton: you write the whole `impl`, and the checks
//! are the programs in `tests/cases/l03/`. Add one edge case of your own.
//! EXPLAIN afterward: Explain why `WindowsMut` can implement `LendingIterator`
//! and cannot implement `Iterator`. Name the line of the program in
//! `tests/cases/l03/two_windows_at_once.rs` that the borrow checker rejects,
//! and say which two borrows collide.
//!
//! COMPILER LAB (optional): Remove `where Self: 'a` from the trait and read
//! the error. Then add a provided method
//! `fn for_each<F: FnMut(Self::Item<'_>)>(self, f: F)` and call it on a
//! `WindowsMut` over a local vector. CONCEPTS.md explains the message you get.

/// An iterator whose items may borrow from the iterator itself.
/// At most one item is alive at a time.
pub trait LendingIterator {
    /// The item, borrowed for the lifetime `'a` of one `next` call.
    type Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>>;

    /// B. Use up the iterator and return how many items it produced.
    fn count(mut self) -> usize
    where
        Self: Sized,
    {
        todo!("Implement this task")
    }
}

/// Overlapping mutable windows of `size` elements: `[0..size]`, then
/// `[1..size + 1]`, and so on, as long as a full window fits.
pub struct WindowsMut<'s, T> {
    slice: &'s mut [T],
    start: usize,
    size: usize,
}

impl<'s, T> WindowsMut<'s, T> {
    /// Panics if `size` is zero.
    pub fn new(slice: &'s mut [T], size: usize) -> Self {
        assert!(size > 0, "a window must have at least one element");
        WindowsMut {
            slice,
            start: 0,
            size,
        }
    }
}

impl<'s, T> LendingIterator for WindowsMut<'s, T> {
    type Item<'a>
        = &'a mut [T]
    where
        Self: 'a;

    /// A. Lend the next window, or return `None` when no full window is left.
    /// Safe code only.
    fn next(&mut self) -> Option<Self::Item<'_>> {
        todo!("Implement this task")
    }
}

/// C. An adapter that stops after `limit` items of the inner lender.
///
/// Write `impl<I: LendingIterator> LendingIterator for Take<I>`. Its items
/// are the inner lender's items. Once `limit` items were produced, `next`
/// returns `None` without calling the inner lender again.
/// Checked by `tests/cases/l03/take_limits_any_lender.rs`.
pub struct Take<I> {
    inner: I,
    left: usize,
}

/// Wraps `inner` so that it lends at most `limit` items.
pub fn take<I: LendingIterator>(inner: I, limit: usize) -> Take<I> {
    Take { inner, left: limit }
}

// Your `impl ... for Take<I>` goes here.

/// D. Lends each line of a text in ASCII upper case.
///
/// Write `Shouter::new(text: &str) -> Shouter<'_>` and
/// `impl LendingIterator for Shouter<'_>` with `&str` items.
/// `new` may allocate once (hint: `text.len()` bytes are always enough).
/// `next` must not allocate: every item is lent from the same buffer.
/// Lines are split as `str::lines` splits them.
/// Checked by `tests/cases/l03/shouter_reuses_its_buffer.rs`.
pub struct Shouter<'t> {
    lines: std::str::Lines<'t>,
    buffer: String,
}

// Your `impl Shouter<'_>` and `impl LendingIterator for Shouter<'_>` go here.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;

    /// A lender for the tests: it lends "3", "2", "1" from one buffer.
    struct Countdown {
        left: u32,
        text: String,
    }

    impl LendingIterator for Countdown {
        type Item<'a> = &'a str;

        fn next(&mut self) -> Option<&str> {
            if self.left == 0 {
                return None;
            }
            self.text = self.left.to_string();
            self.left -= 1;
            Some(&self.text)
        }
    }

    #[test]
    fn a_windows_overlap_and_are_mutable() {
        let mut data = [1, 2, 3, 4, 5];
        let mut windows = WindowsMut::new(&mut data, 3);
        let mut seen = 0;
        while let Some(window) = windows.next() {
            seen += 1;
            assert!(
                seen <= 10,
                "next() must return None after the last full window"
            );
            assert_eq!(window.len(), 3, "every window has exactly `size` elements");
            // Each window adds its first element to its last element.
            window[2] += window[0];
        }
        assert_eq!(seen, 3, "a slice of 5 has 3 windows of length 3");
        assert_eq!(
            data,
            [1, 2, 4, 6, 9],
            "a later window must see what an earlier window wrote"
        );
    }

    #[test]
    fn a_windows_edge_cases() {
        let mut data = [7, 8];
        assert!(
            WindowsMut::new(&mut data, 3).next().is_none(),
            "no window fits when the slice is shorter than `size`"
        );
        let mut whole = WindowsMut::new(&mut data, 2);
        assert_eq!(whole.next(), Some(&mut [7, 8][..]));
        assert_eq!(whole.next(), None);
        assert_eq!(whole.next(), None, "the end must be stable");
        let mut empty: [u8; 0] = [];
        assert!(WindowsMut::new(&mut empty, 1).next().is_none());
        let mut huge = [1_u8, 2];
        assert!(
            WindowsMut::new(&mut huge, usize::MAX).next().is_none(),
            "a huge `size` must give None, not an overflow panic"
        );
    }

    #[test]
    fn a_windows_of_zero_sized_items() {
        let mut units = [(); 4];
        let mut windows = WindowsMut::new(&mut units, 2);
        let mut seen = 0;
        while windows.next().is_some() {
            seen += 1;
            assert!(
                seen <= 10,
                "next() must return None after the last full window"
            );
        }
        assert_eq!(seen, 3);
    }

    #[test]
    fn b_count_uses_up_the_lender() {
        let countdown = Countdown {
            left: 3,
            text: String::new(),
        };
        assert_eq!(countdown.count(), 3);
        let done = Countdown {
            left: 0,
            text: String::new(),
        };
        assert_eq!(done.count(), 0);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Every window of length 2 adds its first element to its second.
        // The windows overlap. What is in `data` afterwards?
        let mut data = [1, 1, 1, 1];
        for start in 0..data.len() - 1 {
            let window = &mut data[start..start + 2];
            window[1] += window[0];
        }
        let p1: [i32; 4] = guess();
        assert_eq!(
            p1, data,
            "P1: the slice after overlapping windows (left: your prediction, right: actual)"
        );

        // P2. How many windows does a slice of 5 elements have, for window
        // lengths 2, 5 and 6?
        let five = [0_u8; 5];
        let p2: (usize, usize, usize) = guess();
        assert_eq!(
            p2,
            (
                five.windows(2).count(),
                five.windows(5).count(),
                five.windows(6).count()
            ),
            "P2: number of windows (left: your prediction, right: actual)"
        );

        // P3. One buffer is cleared and refilled three times. How many
        // different heap addresses does it use?
        let mut buffer = String::with_capacity(16);
        let mut addresses = Vec::new();
        for word in ["ab", "cdef", "g"] {
            buffer.clear();
            buffer.push_str(word);
            addresses.push(buffer.as_ptr() as usize);
        }
        addresses.dedup();
        let p3: usize = guess();
        assert_eq!(
            p3,
            addresses.len(),
            "P3: distinct buffer addresses (left: your prediction, right: actual)"
        );
    }
}
