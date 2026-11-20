//! DRILL M2-06 · Using a vector while it is being drained
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: in which state is the vector between the call of `drain` and the drop of the iterator?

fn main() {
    let mut queue = vec![1, 2, 3, 4];
    let front = queue.drain(..2);
    queue.push(5);
    let taken: Vec<i32> = front.collect();
    assert_eq!(taken, [1, 2]); // KEEP
    assert_eq!(queue, [3, 4, 5]); // KEEP
}
