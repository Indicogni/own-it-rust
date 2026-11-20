//! DRILL 24-04 · Exporting a symbol
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: error
//!
//! Run `cargo drill ch24`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: why can an unmangled symbol name be unsafe?

// FIX: `no_mangle` is an unsafe attribute in edition 2024. An unmangled name
// is global to the whole program: if another library exports the same name,
// the linker may pick either one. Writing `unsafe(...)` says that you checked.
#[unsafe(no_mangle)]
pub extern "C" fn ch24_answer() -> i32 {
    42
}

fn main() {
    assert_eq!(ch24_answer(), 42); // KEEP
}
