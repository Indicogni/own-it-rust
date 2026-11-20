//! DRILL M8-03 · Using a sealed trait from outside
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m8`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does sealing forbid: naming the trait, calling it, or implementing it?

/// Stands for a library crate.
mod units {
    mod sealed {
        pub trait Sealed {}
    }

    /// Sealed: only this library implements it.
    pub trait Unit: sealed::Sealed {
        fn symbol(&self) -> &'static str;
    }

    pub struct Meter;
    pub struct Second;

    impl sealed::Sealed for Meter {}
    impl sealed::Sealed for Second {}

    impl Unit for Meter {
        fn symbol(&self) -> &'static str {
            "m"
        }
    }

    impl Unit for Second {
        fn symbol(&self) -> &'static str {
            "s"
        }
    }
}

use units::{Meter, Second, Unit};

/// A user's generic function with the sealed trait as a bound.
fn label<U: Unit>(value: f64, unit: &U) -> String { // KEEP
    format!("{value} {}", unit.symbol()) // KEEP
}

fn main() {
    assert_eq!(label(1.5, &Meter), "1.5 m"); // KEEP
    let units: Vec<Box<dyn Unit>> = vec![Box::new(Meter), Box::new(Second)]; // KEEP
    let symbols: Vec<&str> = units.iter().map(|unit| unit.symbol()).collect(); // KEEP
    assert_eq!(symbols, ["m", "s"]); // KEEP
}
