//! DRILL 23-03 · Writing through *const
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0594
//!
//! Run `cargo drill ch23`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does the pointer's type promise about writes?

// FIX: a `*const i32` never permits a write, not even in an unsafe block.
// `unsafe` adds a few operations; it does not change what a type allows.
fn main() {
    let mut x = 1;
    let p: *mut i32 = &mut x;
    unsafe {
        *p = 5; // SAFETY: p comes from a live &mut x.
    }
    assert_eq!(x, 5); // KEEP
}
