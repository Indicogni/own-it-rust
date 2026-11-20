//! DRILL 14-04 · Mutating inside an Fn closure
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0594
//!
//! Run `cargo drill ch14`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which call trait does this function promise its caller?

// FIX: the function only needs to call `f` several times, so it should ask for
// `FnMut`, the weakest bound that allows that. `Fn` rejected closures that
// change their captures.
fn call_three_times<F: FnMut()>(mut f: F) {
    f();
    f();
    f();
}

fn main() {
    let mut total = 0;
    call_three_times(|| total += 5);
    assert_eq!(total, 15); // KEEP
}
