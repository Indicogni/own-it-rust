//! DRILL M1-18 · A bare type parameter as the `Self` type
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which types could `T` be, and who owns them?

struct Meters(f64);

impl<T: From<f64>> From<Meters> for T {
    fn from(meters: Meters) -> T {
        T::from(meters.0)
    }
}

fn main() {
    let raw: f64 = Meters(2.5).into();
    assert_eq!(raw, 2.5); // KEEP
}
