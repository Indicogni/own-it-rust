//@ expect: E0499
//@ why: two overlapping mutable windows must never be alive together; each window must keep the iterator borrowed until it is dropped.
use m1_types::l03_lending::{LendingIterator, WindowsMut};

fn main() {
    let mut data = [1, 2, 3, 4];
    let mut windows = WindowsMut::new(&mut data, 3);
    let first = windows.next().unwrap();
    let second = windows.next().unwrap();
    // `first[1]` and `second[0]` would be the same element.
    first[1] = 10;
    second[0] = 20;
}
