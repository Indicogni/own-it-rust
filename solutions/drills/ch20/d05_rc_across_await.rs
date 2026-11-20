//! DRILL 20-05 · Rc held across an await
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: error
//!
//! Run `cargo drill ch20`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which values does the future store while it is suspended?

use std::future::{Future, ready};
use std::rc::Rc;

fn spawn_check<F: Future + Send + 'static>(future: F) -> F {
    future
}

// FIX: a value that is alive at an `.await` is stored inside the future, and
// an `Rc` inside makes the whole future `!Send`. The inner block ends the
// life of the `Rc` before the await, so the future stores only a usize.
// (Calling `drop(shared)` before the await is not enough for the compiler;
// see drill 11. `Arc` instead of `Rc` is the fix when the value really must
// live across the await.)
async fn report() -> usize {
    let len = {
        let shared = Rc::new(vec![1, 2, 3]);
        shared.len()
    };
    ready(()).await;
    len
}

fn main() {
    let task = spawn_check(report()); // KEEP
    drop(task);
}
