//! Two threads increment a counter through a raw pointer, with no synchronisation.
//!
//! Without a sanitizer this program prints some total and exits with 0. Nothing
//! tells you that the number is wrong, or why.

use std::thread;

struct SharedCounter(*mut u64);

// SAFETY: wrong on purpose. Sending the pointer is fine, but the threads then write
// through it at the same time without a lock or an atomic.
unsafe impl Send for SharedCounter {}

fn bump(counter: SharedCounter, times: u32) {
    for _ in 0..times {
        // SAFETY: wrong on purpose, see above: this is a data race.
        unsafe { *counter.0 += 1 };
    }
}

fn main() {
    let cell = Box::into_raw(Box::new(0_u64));
    let first = SharedCounter(cell);
    let second = SharedCounter(cell);
    let a = thread::spawn(move || bump(first, 100_000));
    let b = thread::spawn(move || bump(second, 100_000));
    a.join().unwrap();
    b.join().unwrap();
    // SAFETY: both threads have finished; this is the only access now.
    let total = unsafe { *Box::from_raw(cell) };
    println!("total: {total}");
}
