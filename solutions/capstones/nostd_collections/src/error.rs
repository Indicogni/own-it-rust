//! The one error type of the crate (SPEC section 3).

use core::fmt;

/// A collection or an arena had no room for a value.
///
/// The value is not lost: [`CapacityError::into_inner`] gives it back.
pub struct CapacityError<T> {
    value: T,
}

impl<T> CapacityError<T> {
    /// Wraps the value that did not fit. Only this crate reports the error,
    /// so only this crate can build it.
    pub(crate) const fn new(value: T) -> Self {
        Self { value }
    }

    /// Returns the value that did not fit.
    pub fn into_inner(self) -> T {
        self.value
    }
}

// Written by hand, without a `T: Debug` bound: `Result::unwrap` and
// `Result::expect` need `E: Debug`, and they must work for every element
// type. The rejected value is not printed.
impl<T> fmt::Debug for CapacityError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CapacityError(..)")
    }
}

impl<T> fmt::Display for CapacityError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("insufficient capacity")
    }
}

impl<T> core::error::Error for CapacityError<T> {}
