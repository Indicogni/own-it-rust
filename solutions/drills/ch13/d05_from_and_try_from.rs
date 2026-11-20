//! DRILL 13-05 · From and TryFrom for the same type
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0119
//!
//! Run `cargo drill ch13`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does std already implement once you write From?

#[derive(Debug, PartialEq)]
struct Port(u16);

// FIX: the `From<u16>` impl is gone. `From` gives you `Into`, and `Into` gives you
// `TryFrom` through std's blanket impl, so writing both by hand overlaps. This
// conversion can fail, so `TryFrom` is the only one that should exist.
impl TryFrom<u16> for Port {
    type Error = &'static str;
    fn try_from(n: u16) -> Result<Self, Self::Error> {
        if n == 0 { Err("port 0 is reserved") } else { Ok(Port(n)) }
    }
}

fn main() {
    assert_eq!(Port::try_from(0), Err("port 0 is reserved")); // KEEP
    assert_eq!(Port::try_from(80), Ok(Port(80))); // KEEP
}
