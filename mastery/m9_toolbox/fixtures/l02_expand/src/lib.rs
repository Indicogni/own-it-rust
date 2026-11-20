//! The source that `expanded.rs` was generated from, with
//! `cargo expand --lib > expanded.rs` (cargo-expand 1.0.127).

use std::rc::Rc;

/// Field order matters to the derived comparison.
#[derive(Debug, PartialEq, PartialOrd)]
pub struct Release {
    pub patch: u32,
    pub major: u32,
}

/// A handle that shares its payload.
#[derive(Clone)]
pub struct Shared<T> {
    pub inner: Rc<T>,
}

/// Adds a bonus to `$e`. The macro has a local variable of its own, named `x`.
#[macro_export]
macro_rules! with_bonus {
    ($e:expr) => {{
        let x = 100;
        $e + x
    }};
}

pub fn bonus_demo() -> i32 {
    let x = 1;
    with_bonus!(x)
}
