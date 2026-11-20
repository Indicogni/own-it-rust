//! DRILL M6-10 · Two modules, one symbol name
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: error
//!
//! Run `cargo drill m6`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: Rust paths have modules. What does the linker have?

// FIX: `no_mangle` exports each function under its bare name, and both are
// called `init`. Rust keeps them apart by their paths, but C and the linker
// have one flat namespace: a symbol name exists once in a program. Give each
// export a name with the library's prefix. `export_name` sets the symbol
// without changing the Rust name. This is also why `no_mangle` must be
// written `unsafe(...)`: a symbol that collides with one from another
// library is undefined behavior that no compiler error announces.
mod parser {
    /// Exported for C callers.
    #[unsafe(export_name = "parser_init")]
    pub extern "C" fn init() -> i32 {
        1
    }
}

mod network {
    /// Exported for C callers.
    #[unsafe(export_name = "network_init")]
    pub extern "C" fn init() -> i32 {
        2
    }
}

fn main() {
    assert_eq!(parser::init() + network::init(), 3); // KEEP
}
