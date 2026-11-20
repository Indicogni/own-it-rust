//! DRILL M1-15 · A method that this state does not have
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0599
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what is the type of `valve` on the KEEP line?

use std::marker::PhantomData;

struct Closed;
struct Open;

struct Valve<State> {
    _state: PhantomData<State>,
}

impl Valve<Closed> {
    fn new() -> Self {
        Valve { _state: PhantomData }
    }
    fn open(self) -> Valve<Open> {
        Valve { _state: PhantomData }
    }
}

impl Valve<Open> {
    fn flow(&self) -> u32 {
        10
    }
    fn close(self) -> Valve<Closed> {
        Valve { _state: PhantomData }
    }
}

// FIX: `close` returns a `Valve<Closed>`, and `flow` is defined in
// `impl Valve<Open>` only, so the type has no such method. This is the
// typestate pattern doing its work: reading the flow of a closed valve is a
// compile error, not a run-time check. Keep the valve open.
fn main() {
    let valve = Valve::new().open();
    assert_eq!(valve.flow(), 10); // KEEP
}
