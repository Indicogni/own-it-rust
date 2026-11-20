//! DRILL 04-01 · Use after move
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0382
//!
//! Run `cargo drill ch04`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which binding owns the buffer after line 2?

// FIX: `let saved = route;` moved the String, so `route` was empty afterwards.
// The KEEP line needs both names, so make a second, independent String.
// (`let saved = &route;` also works and allocates nothing.)
fn main() {
    let route = String::from("north");
    let saved = route.clone();
    assert_eq!(format!("{route}/{saved}"), "north/north"); // KEEP
}
