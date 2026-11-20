//! DRILL 23-10 · Moving out through a raw pointer
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0507
//!
//! Run `cargo drill ch23`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many owners of the String's buffer would there be?

// FIX: moving the String out through the pointer would leave two owners of
// one buffer (`name` and `copy`), so the compiler refuses. The KEEP line needs
// both values, so clone: the clone has its own buffer.
fn main() {
    let name = String::from("ferris");
    let p: *const String = &name;
    // SAFETY: `p` comes from a reference to `name`, which is alive; the
    // shared reference made here lasts only for the call to `clone`.
    let copy: String = unsafe { (*p).clone() };
    assert_eq!(copy, name); // KEEP
}
