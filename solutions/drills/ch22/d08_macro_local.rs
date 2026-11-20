//! DRILL 22-08 · Locals declared by a macro
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0425
//!
//! Run `cargo drill ch22`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who wrote the name `x`: the caller or the macro?

// FIX: an `x` written by the macro is a different name from the caller's `x`.
// A name that the caller passes in keeps the caller's context, so the caller
// can use it afterwards.
macro_rules! make_x {
    ($name:ident) => {
        let $name = 1;
    };
}

fn main() {
    make_x!(x);
    assert_eq!(x, 1); // KEEP
}
