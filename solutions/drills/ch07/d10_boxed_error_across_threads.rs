//! DRILL 07-10 · A boxed error that leaves its thread
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill ch07`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what must be true of a value that a spawned thread hands back?

use std::error::Error;
use std::thread;

fn parse(text: &str) -> Result<u32, Box<dyn Error + Send + Sync>> {
    Ok(text.trim().parse()?)
}

fn main() {
    let worker = thread::spawn(|| parse(" 42 ")); // KEEP
    let answer = worker.join().expect("the worker does not panic"); // KEEP
    assert_eq!(answer.ok(), Some(42)); // KEEP
}
