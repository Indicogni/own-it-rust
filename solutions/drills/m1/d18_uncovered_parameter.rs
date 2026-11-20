//! DRILL M1-18 · A bare type parameter as the `Self` type
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0210
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which types could `T` be, and who owns them?

struct Meters(f64);

// FIX: in `impl<T> From<Meters> for T`, the `Self` type is a bare type
// parameter that stands before the local type `Meters`. `T` could be any
// type of any crate, and that crate may already have, or may later add, its
// own `From<Meters>`. The orphan rule forbids an uncovered parameter before
// the first local type. Name the target type: `From<Meters> for f64` is
// allowed, because the local type is a type argument of the trait.
impl From<Meters> for f64 {
    fn from(meters: Meters) -> f64 {
        meters.0
    }
}

fn main() {
    let raw: f64 = Meters(2.5).into();
    assert_eq!(raw, 2.5); // KEEP
}
