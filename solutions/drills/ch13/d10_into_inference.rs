//! DRILL 13-10 · Into needs to know the target
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0283
//!
//! Run `cargo drill ch13`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: into what?

fn main() {
    let small: u8 = 200;
    // FIX: `u8` converts into many types, so `into` needs to be told the target.
    let wide: u16 = small.into();
    assert_eq!(wide + 100, 300); // KEEP
}
