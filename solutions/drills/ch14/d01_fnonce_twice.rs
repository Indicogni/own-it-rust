//! DRILL 14-01 · Calling a consuming closure twice
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0382
//!
//! Run `cargo drill ch14`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does the first call do to `name`?

fn main() {
    let name = String::from("ferris");
    // FIX: returning `name` moved it out, so the closure could run only once.
    // Returning a clone leaves the capture in place, and the closure is `Fn`.
    let take = move || name.clone();
    let first = take();
    let second = take();
    assert_eq!((first.as_str(), second.as_str()), ("ferris", "ferris")); // KEEP
}
