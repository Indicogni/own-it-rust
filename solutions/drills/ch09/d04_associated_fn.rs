//! DRILL 09-04 · Associated functions are not methods
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0599
//!
//! Run `cargo drill ch09`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: does `new` have a `self` parameter?

struct Gauge {
    reading: u8,
}

impl Gauge {
    fn new(reading: u8) -> Gauge {
        Gauge { reading }
    }
}

fn main() {
    let first = Gauge::new(1);
    // FIX: `new` has no `self` parameter, so it is called through the type.
    let second = Gauge::new(5);
    assert_eq!(first.reading + second.reading, 6); // KEEP
}
