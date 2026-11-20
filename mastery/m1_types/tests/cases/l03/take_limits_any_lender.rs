//@ expect: run
//@ why: Take must be a LendingIterator for every inner lender, pass the inner items through, and stop after `limit` items.
use m1_types::l03_lending::{LendingIterator, WindowsMut, take};

/// A lender that counts how often it was asked.
struct Counter<'c> {
    calls: &'c mut u32,
    scratch: u32,
}

impl LendingIterator for Counter<'_> {
    type Item<'a>
        = &'a mut u32
    where
        Self: 'a;

    fn next(&mut self) -> Option<&mut u32> {
        *self.calls += 1;
        self.scratch = *self.calls * 10;
        Some(&mut self.scratch)
    }
}

fn main() {
    // The items of the inner lender come through, still mutable.
    let mut data = [1, 2, 3, 4, 5];
    let mut firsts = take(WindowsMut::new(&mut data, 2), 2);
    let mut seen = 0;
    while let Some(window) = firsts.next() {
        seen += 1;
        assert!(seen <= 10, "Take::next never returned None");
        window[0] *= 10;
    }
    assert_eq!(seen, 2);
    assert_eq!(data, [10, 20, 3, 4, 5]);

    // A limit above the length is harmless, and `count` works on a Take.
    let mut data = [1, 2, 3];
    assert_eq!(take(WindowsMut::new(&mut data, 2), 99).count(), 2);

    // After the limit, the inner lender is not called again.
    let mut calls = 0;
    {
        let mut limited = take(
            Counter {
                calls: &mut calls,
                scratch: 0,
            },
            3,
        );
        let mut last = 0;
        for _ in 0..3 {
            let item = limited.next().expect("the inner lender never ends");
            last = *item;
        }
        assert_eq!(last, 30);
        assert!(limited.next().is_none());
        assert!(limited.next().is_none());
    }
    assert_eq!(
        calls, 3,
        "the inner lender must not be called after the limit"
    );

    assert!(take(WindowsMut::new(&mut [1, 2, 3], 1), 0).next().is_none());
}
