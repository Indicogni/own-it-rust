//! DRILL 22-02 · A macro cannot see the caller's locals
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0425
//!
//! Run `cargo drill ch22`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: where are the names in a macro body looked up?

// FIX: a local name written in a macro body belongs to the macro, so it never
// finds the caller's `tax`. The KEEP line passes one argument, so the macro
// must be defined where `tax` is already in scope: inside `main`, after the
// `let`. (With a free hand, the cleaner design is a second argument:
// `add_tax!(100, tax)`.)
fn main() {
    let tax = 5; // KEEP
    macro_rules! add_tax {
        ($price:expr) => {
            $price + tax
        };
    }
    assert_eq!(add_tax!(100), 105); // KEEP
}
