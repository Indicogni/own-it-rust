//! Scratch space for the optional labs of chapter 20: Writing async code.
//! Run: cargo run --example ch20_scratch
//!
//! Each lesson header in exercises/chapter20/ describes one experiment (its
//! COMPILER LAB line). Type the experiment into this file, run it, and read
//! what the compiler says. A lab often asks for code that must NOT compile:
//! keep it here only while you study the error, then repair it so that this
//! file compiles again.
//!
//! The standard library has no executor, so `run` below drives one future to
//! its end by polling it in a loop. CONCEPTS.md explains it.

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

/// Polls `future` until it is ready and returns its output.
fn run<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

fn main() {
    let answer = run(async { 6 * 7 });
    println!(
        "Chapter 20 scratch: the async block returned {answer}. Add one experiment at a time."
    );
}
