//! DRILL M5-05 · The order a derive compares in
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m5`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: a derive sees the fields as a list of tokens. In which order does `PartialOrd` go through them?

#[derive(Debug, PartialEq, PartialOrd)]
struct Version {
    // The derived comparison goes through the fields in declaration order,
    // so the most important field comes first.
    major: u32,
    minor: u32,
    patch: u32,
}

fn version(major: u32, minor: u32, patch: u32) -> Version {
    Version {
        major,
        minor,
        patch,
    }
}

fn main() {
    let old = version(1, 9, 4); // KEEP
    let new = version(2, 0, 0); // KEEP
    assert!(new > old, "2.0.0 must be newer than 1.9.4"); // KEEP
}
