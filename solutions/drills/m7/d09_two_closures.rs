//! DRILL M7-09 · Two closures, one return type
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0308
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many different types does `impl Fn` stand for in one function?

// FIX: every closure expression has a type of its own, and `impl Fn` stands
// for exactly one type, so the two branches did not match (E0308). One closure
// that captures both values has one type and is still called directly.
// `Box<dyn Fn(u64) -> u64>` would compile too, but a `Box` is not `Copy`, and
// it costs an allocation and an indirect call. (Two closures that capture
// nothing would have compiled: both can turn into the same `fn(u64) -> u64`
// pointer.)
/// A function that adds `amount` or multiplies by it.
fn step(amount: u64, add: bool) -> impl Fn(u64) -> u64 + Copy {
    // One closure that captures both values and decides inside.
    move |x| if add { x + amount } else { x * amount }
}

/// Compiles only for a value that is plain data: a `Box` is not `Copy`.
fn is_copy<T: Copy>(_value: &T) {}

fn main() {
    let double = step(2, false);
    is_copy(&double); // KEEP
    assert_eq!(double(21), 42); // KEEP
    assert_eq!(step(2, true)(21), 23); // KEEP
}
