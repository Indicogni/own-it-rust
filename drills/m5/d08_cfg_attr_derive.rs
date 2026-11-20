//! DRILL M5-08 · A derive behind cfg_attr
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m5`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: `cfg_attr` is resolved before any macro runs. In which builds does this derive exist?

#[cfg_attr(test, derive(Debug))]
struct Job {
    id: u32,
}

fn main() {
    let job = Job { id: 7 }; // KEEP
    assert_eq!(format!("{job:?}"), "Job { id: 7 }"); // KEEP
}
