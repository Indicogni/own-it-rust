//! DRILL M1-10 · A type parameter that no field uses
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0392
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what would the compiler not know about `Id<T>` if this were accepted?

use std::marker::PhantomData;

// FIX: every type parameter must be used in a field, because the fields are
// where the compiler reads the variance of `T`, the auto traits and the drop
// behavior from. `PhantomData<fn() -> T>` is a zero-sized field that says
// "I mention T, but I hold none": covariant, always Send and Sync.
struct Id<T> {
    raw: u32,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    fn new(raw: u32) -> Self {
        Self { raw, _marker: PhantomData }
    }
}

struct User;

fn main() {
    let id = Id::<User>::new(7); // KEEP
    assert_eq!(id.raw, 7); // KEEP
    assert_eq!(size_of::<Id<User>>(), 4); // KEEP
}
