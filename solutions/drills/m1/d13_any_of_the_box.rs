//! DRILL M1-13 · Which value is behind `&dyn Any`?
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: panic
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: `Box<dyn Any>` is itself a 'static type. Which type does `&boxed` turn into a `dyn Any`?

use std::any::Any;

fn describe(value: &dyn Any) -> String {
    match value.downcast_ref::<u32>() {
        Some(number) => format!("number {number}"),
        None => "something else".to_string(),
    }
}

// FIX: the program compiled and then failed its assertion. `&boxed` is a
// `&Box<dyn Any>`, and `Box<dyn Any>` implements `Any` itself, so the
// coercion made a trait object whose concrete type is the BOX. Its type id is
// not that of `u32`. Dereference first: `&*boxed` is the `&dyn Any` that the
// box already holds.
fn main() {
    let boxed: Box<dyn Any> = Box::new(7_u32); // KEEP
    let value: &dyn Any = &*boxed;
    assert_eq!(describe(value), "number 7"); // KEEP
}
