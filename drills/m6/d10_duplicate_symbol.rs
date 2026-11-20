//! DRILL M6-10 · Two modules, one symbol name
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m6`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: Rust paths have modules. What does the linker have?

mod parser {
    /// Exported for C callers.
    #[unsafe(no_mangle)]
    pub extern "C" fn init() -> i32 {
        1
    }
}

mod network {
    /// Exported for C callers.
    #[unsafe(no_mangle)]
    pub extern "C" fn init() -> i32 {
        2
    }
}

fn main() {
    assert_eq!(parser::init() + network::init(), 3); // KEEP
}
