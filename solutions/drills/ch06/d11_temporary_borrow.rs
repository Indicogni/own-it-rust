//! DRILL 06-11 · Temporaries that hold a borrow
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill ch06`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: drain(..) returns an unnamed value that borrows the vector until that
//! value is dropped. Where is it dropped in each of the two functions?

/// Empties a new batch and reports how many jobs it held.
fn flushed() -> usize {
    let mut batch = vec![4, 5, 6];
    batch.drain(..).len()
}

/// Takes the oldest job and discards the others. An empty queue gets job 7.
fn next_job(queue: &mut Vec<u32>) -> u32 {
    // In an if let, the temporary Drain is dropped before the else block runs,
    // so the else block may borrow the queue again.
    if let Some(job) = queue.drain(..).next() {
        job
    } else {
        queue.push(7);
        7
    }
}

fn main() {
    let mut queue = vec![3, 9];
    assert_eq!(flushed(), 3); // KEEP
    assert_eq!(next_job(&mut queue), 3); // KEEP
    assert_eq!(next_job(&mut queue), 7); // KEEP
    assert_eq!(queue, [7]); // KEEP
}
