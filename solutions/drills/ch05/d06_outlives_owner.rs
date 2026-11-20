//! DRILL 05-06 · A reference that outlives its owner
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0597
//!
//! Run `cargo drill ch05`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: when is `name` dropped?

fn main() {
    // FIX: the owner now lives in the outer scope, so it is still alive at
    // every point where `label` is used.
    let name = String::from("probe");
    let label: &String;
    {
        label = &name;
    }
    assert_eq!(label, "probe"); // KEEP
}
