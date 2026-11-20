//! DRILL 14-07 · Passing a consuming closure where Fn is required
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0525
//!
//! Run `cargo drill ch14`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does the closure body do with `log`?

fn run_twice<F: Fn() -> usize>(f: F) -> usize {
    f() + f()
}

fn main() {
    let log = vec![String::from("boot"), String::from("ready")];
    let summarize = move || {
        // FIX: borrow the captured vector instead of moving it out of the
        // closure. The body now only reads its capture, so the closure is `Fn`.
        let owned = &log;
        owned.len()
    };
    let total = run_twice(summarize);
    assert_eq!(total, 4); // KEEP
}
